package dev.orchid.voice

import android.content.Context
import android.annotation.SuppressLint
import android.media.AudioDeviceInfo
import android.media.AudioManager

/** Connected devices and stable preferences shared by capture, playback and their selectors. */
@SuppressLint("InlinedApi") // Device type comparisons are safe on older versions, which never report newer types.
class AudioDevices(context: Context) {
    private val audio = context.getSystemService(AudioManager::class.java)
    data class Choice(val key: String, val label: String)
    fun inputs(): List<AudioDeviceInfo> = audio.getDevices(AudioManager.GET_DEVICES_INPUTS).filter {
        it.type in listOf(AudioDeviceInfo.TYPE_BUILTIN_MIC, AudioDeviceInfo.TYPE_WIRED_HEADSET,
            AudioDeviceInfo.TYPE_USB_DEVICE, AudioDeviceInfo.TYPE_USB_HEADSET,
            AudioDeviceInfo.TYPE_BLUETOOTH_SCO, AudioDeviceInfo.TYPE_BLE_HEADSET)
    }
    fun outputs(): List<AudioDeviceInfo> = audio.getDevices(AudioManager.GET_DEVICES_OUTPUTS).filter {
        it.type !in listOf(AudioDeviceInfo.TYPE_TELEPHONY, AudioDeviceInfo.TYPE_REMOTE_SUBMIX,
            AudioDeviceInfo.TYPE_BLUETOOTH_SCO)
    }
    fun input(key: String): AudioDeviceInfo? = when (key) {
        "phone" -> inputs().firstOrNull { it.type == AudioDeviceInfo.TYPE_BUILTIN_MIC }
        "headset" -> inputs().firstOrNull { it.type in listOf(AudioDeviceInfo.TYPE_WIRED_HEADSET, AudioDeviceInfo.TYPE_USB_DEVICE, AudioDeviceInfo.TYPE_USB_HEADSET) }
            ?: inputs().firstOrNull { bluetoothMic(it) }
        else -> inputs().firstOrNull { key(it) == key }
    }
    fun output(key: String): AudioDeviceInfo? = outputs().firstOrNull { key(it) == key }
    fun choices(input: Boolean, selected: String): List<Choice> {
        val result = mutableListOf(Choice(if (input) "phone" else "automatic", if (input) "Phone microphone" else "Automatic · Android media output"))
        (if (input) inputs().filter { it.type != AudioDeviceInfo.TYPE_BUILTIN_MIC } else outputs()).forEach {
            result.add(Choice(key(it), label(it)))
        }
        if (result.none { it.key == selected }) result.add(Choice(selected, "${label(selected)} · disconnected"))
        return result.distinctBy { it.key }
    }
    companion object {
        fun key(device: AudioDeviceInfo): String {
            val address = if (device.type in listOf(AudioDeviceInfo.TYPE_USB_DEVICE, AudioDeviceInfo.TYPE_USB_HEADSET)) "" else device.address
            return "${device.type}|$address|${device.productName}"
        }
        fun bluetoothMic(device: AudioDeviceInfo?) = device?.type in listOf(AudioDeviceInfo.TYPE_BLUETOOTH_SCO, AudioDeviceInfo.TYPE_BLE_HEADSET)
        fun label(device: AudioDeviceInfo): String {
            val kind = when (device.type) {
                AudioDeviceInfo.TYPE_BUILTIN_MIC -> "Phone microphone"
                AudioDeviceInfo.TYPE_BUILTIN_SPEAKER -> "Phone speaker"
                AudioDeviceInfo.TYPE_BUILTIN_EARPIECE -> "Phone earpiece"
                AudioDeviceInfo.TYPE_USB_DEVICE, AudioDeviceInfo.TYPE_USB_HEADSET -> "USB"
                AudioDeviceInfo.TYPE_BLUETOOTH_SCO -> "Bluetooth microphone"
                AudioDeviceInfo.TYPE_BLUETOOTH_A2DP, AudioDeviceInfo.TYPE_BLE_HEADSET, AudioDeviceInfo.TYPE_BLE_SPEAKER -> "Bluetooth"
                else -> "Wired"
            }
            return if (kind.startsWith("Phone")) kind else "$kind · ${device.productName}"
        }
        fun label(key: String) = when (key) {
            "phone" -> "Phone microphone"
            "headset" -> "External microphone"
            "automatic" -> "Automatic"
            else -> when (key.substringBefore('|').toIntOrNull()) {
                AudioDeviceInfo.TYPE_BUILTIN_SPEAKER -> "Phone speaker"
                AudioDeviceInfo.TYPE_BUILTIN_EARPIECE -> "Phone earpiece"
                else -> key.substringAfterLast('|')
            }
        }
    }
}
