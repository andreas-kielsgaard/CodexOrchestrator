package dev.orchid.voice

/** Only activation and confirmed relevant input renew the listening window. */
class ListeningInactivity {
    private var lastRelevantAt: Long? = null
    @Synchronized fun activate(now: Long) { lastRelevantAt = now }
    @Synchronized fun receive(type: String, now: Long) {
        if (type == "relevant_input" && lastRelevantAt != null) lastRelevantAt = now
    }
    @Synchronized fun remainingSeconds(now: Long, timeoutSeconds: Int): Long? =
        lastRelevantAt?.let { ((timeoutSeconds * 1000L - (now - it)).coerceAtLeast(0) + 999) / 1000 }
    @Synchronized fun stop() { lastRelevantAt = null }
}
