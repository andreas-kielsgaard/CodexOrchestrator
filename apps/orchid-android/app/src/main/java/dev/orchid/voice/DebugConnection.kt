package dev.orchid.voice

import android.app.Activity
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.provider.Settings
import android.util.Log
import java.net.Inet4Address
import java.net.InetSocketAddress
import java.net.NetworkInterface
import java.net.Socket
import java.util.Collections
import java.util.concurrent.Executors

/** A setting write is never considered success: a local listener must also be discovered. */
@Suppress("DEPRECATION")
class DebugConnection(private val activity: Activity, onStatus: (String, String?, Boolean) -> Unit) {
    private val report: (String, String?, Boolean) -> Unit = { message, endpoint, busy ->
        Log.i("OrchidDebug", message)
        onStatus(message, endpoint, busy)
    }
    private val handler = Handler(Looper.getMainLooper())
    private val worker = Executors.newSingleThreadExecutor()
    private val wifi = activity.applicationContext.getSystemService(WifiManager::class.java)
    private val nsd = activity.getSystemService(NsdManager::class.java)
    private var reservation: WifiManager.LocalOnlyHotspotReservation? = null
    private var discovery: NsdManager.DiscoveryListener? = null
    private var generation = 0
    private var running = false
    private var resolving = false
    private var writes = 0
    private var deadline = 0L

    fun start() {
        release()
        running = true
        writes = 0
        deadline = SystemClock.elapsedRealtime() + 120_000
        val attempt = generation
        report("Checking for an existing debugging listener…", null, true)
        handler.postDelayed({ if (active(attempt)) fail("No debugging listener was found after two minutes. Android may reject mobile-only startup on this device. Setting writes: $writes.") }, 120_000)
        worker.execute {
            val existing = listening(5555)
            handler.post {
                if (!active(attempt)) return@post
                if (existing) found(5555)
                else {
                    discover(attempt)
                    if (!active(attempt)) return@post
                    if (wifi.connectionInfo.networkId != -1) nudge(attempt)
                    else hotspot(attempt)
                }
            }
        }
    }

    private fun hotspot(attempt: Int) {
        report("Starting temporary local hotspot. Keep Orchid visible…", null, true)
        try {
            wifi.startLocalOnlyHotspot(object : WifiManager.LocalOnlyHotspotCallback() {
                override fun onStarted(value: WifiManager.LocalOnlyHotspotReservation) {
                    if (!active(attempt)) { value.close(); return }
                    reservation = value
                    nudge(attempt)
                }
                override fun onFailed(reason: Int) {
                    if (!active(attempt)) return
                    val detail = when (reason) {
                        ERROR_INCOMPATIBLE_MODE -> "Incompatible Wi-Fi mode. Turn Mobile Hotspot off and keep Orchid visible, then retry."
                        ERROR_TETHERING_DISALLOWED -> "Hotspot creation is disabled by phone policy."
                        ERROR_NO_CHANNEL -> "No Wi-Fi channel is available for the temporary hotspot."
                        else -> "Android refused the temporary hotspot (error $reason)."
                    }
                    fail(detail)
                }
                override fun onStopped() { if (active(attempt)) fail("Android stopped the temporary hotspot before debugging started.") }
            }, handler)
        } catch (error: SecurityException) { fail("Hotspot permission is missing: ${error.message}") }
        catch (error: Exception) { fail("Could not create the temporary hotspot: ${error.message}") }
    }

    private fun nudge(attempt: Int) {
        report("Requesting wireless debugging; waiting for an actual listener…", null, true)
        val pulse = object : Runnable {
            override fun run() {
                if (!active(attempt)) return
                if (SystemClock.elapsedRealtime() >= deadline) return
                try {
                    if (Settings.Global.getInt(activity.contentResolver, "adb_wifi_enabled", 0) == 0) {
                        if (!Settings.Global.putInt(activity.contentResolver, "adb_wifi_enabled", 1)) {
                            fail("Android refused the debugging setting. Check the one-time permission setup."); return
                        }
                        writes++
                    }
                } catch (error: Exception) { fail("Could not request debugging: ${error.message}"); return }
                handler.postDelayed(this, 20)
            }
        }
        handler.post(pulse)
    }

    private fun discover(attempt: Int) {
        val listener = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(type: String) {}
            override fun onDiscoveryStopped(type: String) {}
            override fun onServiceLost(info: NsdServiceInfo) {}
            override fun onStartDiscoveryFailed(type: String, error: Int) { handler.post { if (active(attempt)) fail("Debugging discovery failed (error $error).") } }
            override fun onStopDiscoveryFailed(type: String, error: Int) {}
            override fun onServiceFound(info: NsdServiceInfo) {
                handler.post {
                    if (!active(attempt) || resolving) return@post
                    resolving = true
                    try {
                        nsd.resolveService(info, object : NsdManager.ResolveListener {
                            override fun onResolveFailed(service: NsdServiceInfo, error: Int) { handler.post { if (active(attempt)) resolving = false } }
                            override fun onServiceResolved(service: NsdServiceInfo) {
                                handler.post {
                                    if (!active(attempt)) return@post
                                    worker.execute {
                                        val local = addresses().any { it == service.host } && service.port in 1..65535 && listening(service.port)
                                        handler.post {
                                            if (!active(attempt)) return@post
                                            resolving = false
                                            if (local) found(service.port)
                                        }
                                    }
                                }
                            }
                        })
                    } catch (error: Exception) { resolving = false }
                }
            }
        }
        discovery = listener
        try { nsd.discoverServices("_adb-tls-connect._tcp.", NsdManager.PROTOCOL_DNS_SD, listener) }
        catch (error: Exception) { fail("Could not discover debugging: ${error.message}") }
    }

    private fun found(port: Int) {
        running = false
        handler.removeCallbacksAndMessages(null)
        stopDiscovery()
        val address = addresses().filterIsInstance<Inet4Address>().firstOrNull {
            val bytes = it.address
            (bytes[0].toInt() and 255) == 100 && (bytes[1].toInt() and 255) in 64..127
        }?.hostAddress
        val endpoint = address?.let { "$it:$port" }
        report(if (endpoint == null) "A local listener is open on port $port, but no Tailscale IPv4 address was found. Connect Tailscale and retry."
            else "Listener found: $endpoint\nThe computer must connect and authenticate to verify ADB access. Keep this screen open until connected; closing it releases Orchid’s temporary hotspot.", endpoint, false)
    }

    private fun addresses() = runCatching {
        Collections.list(NetworkInterface.getNetworkInterfaces()).flatMap { Collections.list(it.inetAddresses) }
    }.getOrDefault(emptyList())

    private fun listening(port: Int) = runCatching {
        Socket().use { it.connect(InetSocketAddress("127.0.0.1", port), 400) }; true
    }.getOrDefault(false)

    private fun active(attempt: Int) = attempt == generation && running
    private fun stopDiscovery() { discovery?.let { runCatching { nsd.stopServiceDiscovery(it) } }; discovery = null }
    private fun release() {
        generation++; running = false; resolving = false
        handler.removeCallbacksAndMessages(null)
        stopDiscovery()
        reservation?.let { runCatching { it.close() } }; reservation = null
    }
    private fun fail(message: String) { release(); report(message, null, false) }
    fun cancel() { release(); report("Stopped. Any existing ADB TCP listener was left unchanged.", null, false) }
    fun close() { release(); worker.shutdownNow() }
}
