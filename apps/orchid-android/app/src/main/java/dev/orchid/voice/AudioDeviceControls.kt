package dev.orchid.voice

import android.content.Context
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Handler
import android.os.Looper
import android.view.View
import android.widget.*

/** The same independent selectors appear in Settings and Status and follow device hotplug. */
class AudioDeviceControls(context: Context, private val settings: ConnectionSettings,
    private val microphoneChanged: (String) -> Unit, private val outputChanged: (String) -> Unit) : LinearLayout(context) {
    private val audio = context.getSystemService(AudioManager::class.java)
    private val inventory = AudioDevices(context)
    private val devices = object : AudioDeviceCallback() {
        override fun onAudioDevicesAdded(added: Array<AudioDeviceInfo>) { render() }
        override fun onAudioDevicesRemoved(removed: Array<AudioDeviceInfo>) { render() }
    }
    init { orientation = VERTICAL; render() }
    private fun render() {
        removeAllViews()
        selector("Microphone", true, settings.microphone) { microphoneChanged(it); render() }
        selector("Audio output", false, settings.audioOutput) { outputChanged(it); render() }
        if (AudioDevices.bluetoothMic(inventory.input(settings.microphone)))
            addView(PhoneTheme.text(context, "Bluetooth microphones use call audio. Android may also route playback through the headset in call mode.", 13f, PhoneTheme.secondary))
        if (inventory.input(settings.microphone) == null)
            addView(PhoneTheme.text(context, "Selected microphone disconnected · using the phone microphone until it returns.", 13f, PhoneTheme.accent))
        if (settings.audioOutput != "automatic" && inventory.output(settings.audioOutput) == null)
            addView(PhoneTheme.text(context, "Selected output disconnected · using Android media output until it returns.", 13f, PhoneTheme.accent))
    }
    private fun selector(label: String, input: Boolean, selected: String, changed: (String) -> Unit) {
        addView(PhoneTheme.text(context, label, 13f, PhoneTheme.secondary))
        val choices = inventory.choices(input, selected)
        val selectedIndex = choices.indexOfFirst { it.key == selected }.coerceAtLeast(0)
        addView(PhoneTheme.button(context, choices[selectedIndex].label + " ▾") {
            android.app.AlertDialog.Builder(context).setTitle(label)
                .setSingleChoiceItems(choices.map { it.label }.toTypedArray(), selectedIndex) { dialog, position ->
                    dialog.dismiss()
                    val key = choices[position].key
                    if (key != (if (input) settings.microphone else settings.audioOutput)) changed(key)
                }.setNegativeButton("Cancel", null).show()
        }.apply { contentDescription = label }, LayoutParams(-1, -2))
    }
    override fun onAttachedToWindow() { super.onAttachedToWindow(); audio.registerAudioDeviceCallback(devices, Handler(Looper.getMainLooper())); render() }
    override fun onDetachedFromWindow() { audio.unregisterAudioDeviceCallback(devices); super.onDetachedFromWindow() }
}
