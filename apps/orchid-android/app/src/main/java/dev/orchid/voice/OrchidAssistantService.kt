package dev.orchid.voice

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.service.voice.VoiceInteractionService
import android.service.voice.VoiceInteractionSession
import android.service.voice.VoiceInteractionSessionService

class OrchidAssistantService : VoiceInteractionService() {
    override fun onLaunchVoiceAssistFromKeyguard() {
        startActivity(Intent(this, AssistantActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }
}

class OrchidAssistantSessionService : VoiceInteractionSessionService() {
    override fun onNewSession(args: Bundle?): VoiceInteractionSession = OrchidAssistantSession(this)
}

class OrchidAssistantSession(context: Context) : VoiceInteractionSession(context) {
    override fun onPrepareShow(args: Bundle?, flags: Int) {
        super.onPrepareShow(args, flags)
        setUiEnabled(false)
    }
    override fun onShow(args: Bundle?, flags: Int) {
        super.onShow(args, flags)
        // Ignore screen context and caller extras; activation only starts listening.
        startAssistantActivity(Intent(context, AssistantActivity::class.java))
    }
}
