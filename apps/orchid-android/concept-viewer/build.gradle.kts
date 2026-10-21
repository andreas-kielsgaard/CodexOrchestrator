plugins { id("com.android.application") }

android {
    namespace = "dev.orchid.concepts"
    compileSdk = 36
    defaultConfig {
        applicationId = "dev.orchid.concepts"
        minSdk = 29
        targetSdk = 35
        versionCode = 2
        versionName = "0.2"
        testInstrumentationRunner = "dev.orchid.concepts.ViewerProbe"
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    sourceSets.getByName("main").assets.srcDir(layout.buildDirectory.dir("generated/concept-assets").get().asFile)
}

val stageConcepts by tasks.registering(Sync::class) {
    from(rootProject.layout.projectDirectory.dir("../../docs/voice-sessions/visual-exploration"))
    include("01-workflow-canvas.png", "02-live-trace.png", "03-extension-workspace.png",
        "04-prototype-workflow.png", "05-prototype-node.png", "06-prototype-activity.png")
    into(layout.buildDirectory.dir("generated/concept-assets"))
}
tasks.named("preBuild").configure { dependsOn(stageConcepts) }
