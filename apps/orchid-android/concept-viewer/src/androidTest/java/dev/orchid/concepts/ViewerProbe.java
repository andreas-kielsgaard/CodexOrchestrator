package dev.orchid.concepts;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Intent;
import android.os.Bundle;
import android.os.SystemClock;
import android.widget.TextView;

/** Exercises the packaged viewer and assets on the device, without touching Orchid Voice. */
public class ViewerProbe extends Instrumentation {
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        MainActivity activity = null;
        try {
            Intent launch = new Intent(Intent.ACTION_MAIN).setClassName(getTargetContext(), MainActivity.class.getName())
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            activity = (MainActivity) startActivitySync(launch);
            MainActivity viewer = activity;
            waitForImage(viewer);
            checkTitle(viewer, "1 / 3");
            for (int page = 2; page <= 3; page++) {
                runOnMainSync(() -> viewer.findViewById(R.id.next).performClick());
                waitForImage(viewer);
                checkTitle(viewer, page + " / 3");
            }
            runOnMainSync(() -> viewer.findViewById(R.id.previous).performClick());
            waitForImage(viewer);
            checkTitle(viewer, "2 / 3");
            float[] before = new float[1];
            runOnMainSync(() -> { before[0] = viewer.image.getScale(); viewer.findViewById(R.id.zoom).performClick(); });
            SystemClock.sleep(800);
            if (scale(viewer) <= before[0]) throw new AssertionError("Zoom did not increase image scale");
            runOnMainSync(() -> viewer.findViewById(R.id.fit).performClick());
            waitForImage(viewer);
            SystemClock.sleep(800);
            if (Math.abs(scale(viewer) - before[0]) > 0.05f) throw new AssertionError("Fit did not restore image scale");
            result.putString("viewer", "All three bundled concepts rendered; Next, Previous, Zoom and Fit passed");
            finish(Activity.RESULT_OK, result);
        } catch (Throwable error) {
            result.putString("failure", error.toString());
            finish(Activity.RESULT_CANCELED, result);
        } finally { if (activity != null) { MainActivity viewer = activity; runOnMainSync(viewer::finish); } }
    }
    private void waitForImage(MainActivity viewer) {
        long deadline = SystemClock.elapsedRealtime() + 8000;
        while (!viewer.imageLoaded && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(50);
        if (!viewer.imageLoaded) throw new AssertionError("Concept image did not load");
        waitForIdleSync();
        int[] height = new int[1];
        runOnMainSync(() -> height[0] = viewer.image.getContentHeight());
        if (height[0] <= 0) throw new AssertionError("Concept has no rendered content");
    }
    private void checkTitle(MainActivity viewer, String prefix) {
        String[] title = new String[1];
        runOnMainSync(() -> title[0] = ((TextView)viewer.findViewById(R.id.concept_title)).getText().toString());
        if (!title[0].startsWith(prefix)) throw new AssertionError("Wrong concept displayed");
    }
    private float scale(MainActivity viewer) {
        float[] value = new float[1];
        runOnMainSync(() -> value[0] = viewer.image.getScale());
        return value[0];
    }
}
