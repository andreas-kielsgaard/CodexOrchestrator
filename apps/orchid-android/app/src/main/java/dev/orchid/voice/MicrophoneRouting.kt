package dev.orchid.voice

import android.content.Context
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.media.AudioRecord
import android.os.Build
import android.os.Handler
import android.os.Looper

/** Pins capture independently; communication mode is used only for a Bluetooth microphone. */
class MicrophoneRouting(context: Context, private val failed: (String) -> Unit) {
    private val audio = context.getSystemService(AudioManager::class.java)
    private val settings = ConnectionSettings(context)
    private val inventory = AudioDevices(context)
    private var recorder: AudioRecord? = null
    val preferredInput: AudioDeviceInfo?
        get() = inventory.input(settings.microphone) ?: inventory.input("phone")
    val usesBluetooth get() = AudioDevices.bluetoothMic(preferredInput)
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
    @Synchronized fun attach(value: AudioRecord) { recorder = value; update() }

    @Synchronized private fun update() {
        if (!active) return
        val input = preferredInput
        if (!AudioDevices.bluetoothMic(input)) release()
        else if (Build.VERSION.SDK_INT >= 31) {
            val available = audio.availableCommunicationDevices.filter {
                it.type == AudioDeviceInfo.TYPE_BLUETOOTH_SCO || it.type == AudioDeviceInfo.TYPE_BLE_HEADSET
            }
            val headset = available.firstOrNull { it.address == input?.address && it.type == input.type }
            if (headset == null) release()
            else if (headset.id != selected) {
                if (!ownsMode) { audio.mode = AudioManager.MODE_IN_COMMUNICATION; ownsMode = true }
                if (audio.setCommunicationDevice(headset)) selected = headset.id
                else { release(); failed("Unable to select headset microphone") }
            }
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
        if (recorder?.setPreferredDevice(input) == false) failed("Unable to select microphone")
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
        recorder = null
        release()
    }
}
