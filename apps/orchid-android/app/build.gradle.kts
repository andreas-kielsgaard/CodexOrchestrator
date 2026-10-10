plugins { id("com.android.application") }
android {
    namespace = "dev.orchid.voice"
    compileSdk = 36
    defaultConfig { applicationId = "dev.orchid.voice"; minSdk = 29; targetSdk = 35; versionCode = 1; versionName = "0.1" }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    defaultConfig { testInstrumentationRunner = "dev.orchid.voice.ConnectionProbeInstrumentation" }
}
dependencies {
    implementation("androidx.drawerlayout:drawerlayout:1.2.0")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.github.gkonovalov.android-vad:webrtc:2.0.10")
}
