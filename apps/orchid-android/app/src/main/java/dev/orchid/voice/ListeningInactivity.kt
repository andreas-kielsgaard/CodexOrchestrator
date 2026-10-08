package dev.orchid.voice

/** Only activation and confirmed relevant input renew the listening window. */
class ListeningInactivity {
    private var lastRelevantAt: Long? = null
    private var followupUntil: Long? = null
    @Synchronized fun activate(now: Long) { lastRelevantAt = now; followupUntil = null }
    @Synchronized fun receive(type: String, now: Long) {
        if (type in listOf("relevant_input", "input_continuation") && lastRelevantAt != null) { lastRelevantAt = now; followupUntil = null }
    }
    @Synchronized fun delivered(now: Long, seconds: Int) { if (lastRelevantAt != null) followupUntil = now + seconds * 1000L }
    @Synchronized fun followingUp() = followupUntil != null
    @Synchronized fun remainingSeconds(now: Long, timeoutSeconds: Int): Long? =
        lastRelevantAt?.let { (((followupUntil ?: (it + timeoutSeconds * 1000L)) - now).coerceAtLeast(0) + 999) / 1000 }
    @Synchronized fun stop() { lastRelevantAt = null; followupUntil = null }
}
