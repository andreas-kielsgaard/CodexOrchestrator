package dev.orchid.voice

/** Only a fresh, completed sentence currently classified as addressed may interrupt. */
class AddressedSpeechInterruption {
    private val confirmed = mutableMapOf<String, Long>()
    @Synchronized fun remember(id: String, revision: Long) { confirmed[id] = maxOf(confirmed[id] ?: 0L, revision) }
    @Synchronized fun accept(id: String, revision: Long, addressed: Boolean, sentenceComplete: Boolean): Boolean {
        if (!addressed || !sentenceComplete || revision <= (confirmed[id] ?: 0L)) return false
        confirmed[id] = revision
        return true
    }
}
