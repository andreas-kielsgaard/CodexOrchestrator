package dev.orchid.voice

import android.content.Context
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build
import android.os.Handler
import android.os.Looper

/** Owns headset communication routing only while Orchid captures audio. */
class MicrophoneRouting(context: Context, private val failed: (String) -> Unit) {
    private val audio = context.getSystemService(AudioManager::class.java)
    private val phoneMicrophone = ConnectionSettings(context).microphone == "phone"
    val preferredInput: AudioDeviceInfo?
        get() = audio.getDevices(AudioManager.GET_DEVICES_INPUTS).firstOrNull {
            if (phoneMicrophone) it.type == AudioDeviceInfo.TYPE_BUILTIN_MIC
            else it.type in listOf(AudioDeviceInfo.TYPE_WIRED_HEADSET, AudioDeviceInfo.TYPE_USB_HEADSET, AudioDeviceInfo.TYPE_USB_DEVICE)
        }
    private val handler = Handler(Looper.getMainLooper())
    private var active = false
    private var selected: Int? = null
    private var ownsMode = false
    private val devices = object : AudioDeviceCallback() {
        override fun onAudioDevicesAdded(added: Array<AudioDeviceInfo>) { update() }
        override fun onAudioDevicesRemoved(removed: Array<AudioDeviceInfo>) { update() }
    }

    @Synchronized fun start() {
        active = true
        audio.registerAudioDeviceCallback(devices, handler)
        update()
    }

    @Synchronized private fun update() {
        if (!active || phoneMicrophone) return
        if (preferredInput != null) { release(); return }
        if (Build.VERSION.SDK_INT >= 31) {
            val available = audio.availableCommunicationDevices.filter {
                it.type == AudioDeviceInfo.TYPE_BLUETOOTH_SCO || it.type == AudioDeviceInfo.TYPE_BLE_HEADSET
            }
            val headset = available.firstOrNull { it.id == audio.communicationDevice?.id } ?: available.firstOrNull()
            if (headset?.id == selected) return
            if (headset == null) { release(); return }
            if (!ownsMode) { audio.mode = AudioManager.MODE_IN_COMMUNICATION; ownsMode = true }
            if (audio.setCommunicationDevice(headset)) selected = headset.id
            else { release(); failed("Unable to select headset microphone") }
        } else {
            val connected = audio.getDevices(AudioManager.GET_DEVICES_INPUTS).any { it.type == AudioDeviceInfo.TYPE_BLUETOOTH_SCO }
            if (connected && !ownsMode) {
                audio.mode = AudioManager.MODE_IN_COMMUNICATION; ownsMode = true
                @Suppress("DEPRECATION")
                audio.startBluetoothSco()
                @Suppress("DEPRECATION")
                audio.isBluetoothScoOn = true
            } else if (!connected) release()
        }
    }

    private fun release() {
        if (Build.VERSION.SDK_INT >= 31) {
            if (selected != null || ownsMode) audio.clearCommunicationDevice()
        } else if (ownsMode) {
            @Suppress("DEPRECATION")
            audio.stopBluetoothSco()
            @Suppress("DEPRECATION")
            audio.isBluetoothScoOn = false
        }
        selected = null
        if (ownsMode) audio.mode = AudioManager.MODE_NORMAL
        ownsMode = false
    }

    @Synchronized fun close() {
        if (!active) return
        active = false
        audio.unregisterAudioDeviceCallback(devices)
        release()
    }
}
