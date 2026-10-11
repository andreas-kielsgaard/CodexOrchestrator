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
        var displayed = selected
        addView(Spinner(context).apply {
            contentDescription = label
            adapter = object : ArrayAdapter<String>(context, android.R.layout.simple_spinner_item, choices.map { it.label }) {
                override fun getView(position: Int, convertView: View?, parent: android.view.ViewGroup): View =
                    super.getView(position, convertView, parent).apply { (this as TextView).setTextColor(PhoneTheme.primary) }
            }.apply { setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item) }
            minimumHeight = PhoneTheme.dp(context, 48)
            setSelection(choices.indexOfFirst { it.key == selected }.coerceAtLeast(0))
            onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
                override fun onNothingSelected(parent: AdapterView<*>?) {}
                override fun onItemSelected(parent: AdapterView<*>?, view: View?, position: Int, id: Long) {
                    val key = choices[position].key
                    if (key == displayed) return
                    displayed = key
                    if (key != (if (input) settings.microphone else settings.audioOutput)) changed(key)
                }
            }
        }, LayoutParams(-1, -2))
    }
    override fun onAttachedToWindow() { super.onAttachedToWindow(); audio.registerAudioDeviceCallback(devices, Handler(Looper.getMainLooper())); render() }
    override fun onDetachedFromWindow() { audio.unregisterAudioDeviceCallback(devices); super.onDetachedFromWindow() }
}
