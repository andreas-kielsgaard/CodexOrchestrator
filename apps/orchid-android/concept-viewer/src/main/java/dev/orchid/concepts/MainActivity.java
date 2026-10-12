package dev.orchid.concepts;

import android.app.Activity;
import android.content.pm.ActivityInfo;
import android.content.res.Configuration;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

/** Offline viewer for desktop mockups, with their original proportions preserved. */
public class MainActivity extends Activity {
    private static final String[] FILES = {"01-workflow-canvas.png", "02-live-trace.png", "03-extension-workspace.png"};
    private static final String[] TITLES = {"Workflow canvas", "Live interaction trace", "Voice extension workspace"};
    private static final String[] NOTES = {
        "Layout sketch: Digest should be labelled Agent.",
        "Layout sketch: extra extension names are placeholders.",
        "Layout sketch: Router delegates; Digest observes and requests output."
    };
    private int index;
    private TextView title, note;
    private Button previous, next;
    private boolean landscape;
    WebView image;
    volatile boolean imageLoaded;

    @Override public void onCreate(Bundle saved) {
        super.onCreate(saved);
        landscape = getResources().getConfiguration().orientation == Configuration.ORIENTATION_LANDSCAPE;
        index = saved == null ? 0 : saved.getInt("concept", 0);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(Color.rgb(20, 24, 25));
        root.setOnApplyWindowInsetsListener((view, insets) -> {
            if (Build.VERSION.SDK_INT >= 30) {
                android.graphics.Insets bars = insets.getInsets(WindowInsets.Type.systemBars());
                view.setPadding(bars.left, bars.top, bars.right, bars.bottom);
            } else {
                view.setPadding(insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(),
                    insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom());
            }
            return insets;
        });
        TextView heading = text("Orchid Concepts · Desktop designs", 18);
        if (!landscape) root.addView(heading);
        title = text("", 15);
        title.setId(R.id.concept_title);
        root.addView(title);
        TextView help = text("Pinch to zoom · Drag to pan · Wide for landscape", 12);
        help.setTextColor(Color.rgb(162, 176, 168));
        if (!landscape) root.addView(help);

        image = new WebView(this);
        image.setId(R.id.concept_image);
        image.setBackgroundColor(Color.rgb(20, 24, 25));
        image.getSettings().setSupportZoom(true);
        image.getSettings().setBuiltInZoomControls(true);
        image.getSettings().setDisplayZoomControls(false);
        image.getSettings().setUseWideViewPort(true);
        image.getSettings().setLoadWithOverviewMode(true);
        image.getSettings().setAllowFileAccess(true);
        image.getSettings().setBlockNetworkLoads(true);
        image.setWebViewClient(new WebViewClient() {
            @Override public void onPageFinished(WebView view, String url) { imageLoaded = true; }
        });
        root.addView(image, new LinearLayout.LayoutParams(-1, 0, 1));

        note = text("", 12);
        note.setTextColor(Color.rgb(189, 200, 194));
        if (!landscape) root.addView(note);
        LinearLayout controls = new LinearLayout(this);
        previous = button("Previous", R.id.previous, controls, () -> show(index - 1));
        next = button("Next", R.id.next, controls, () -> show(index + 1));
        button("Fit", R.id.fit, controls, () -> show(index));
        button("Zoom +", R.id.zoom, controls, () -> image.zoomIn());
        button(landscape ? "Portrait" : "Wide", R.id.wide, controls, () -> setRequestedOrientation(landscape
            ? ActivityInfo.SCREEN_ORIENTATION_PORTRAIT : ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE));
        root.addView(controls);
        setContentView(root);
        if (Build.VERSION.SDK_INT >= 30) {
            WindowInsetsController bars = getWindow().getInsetsController();
            if (landscape) {
                bars.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
                bars.hide(WindowInsets.Type.systemBars());
            } else bars.show(WindowInsets.Type.systemBars());
        }
        show(index);
    }

    private TextView text(String value, int size) {
        TextView view = new TextView(this);
        view.setText(value);
        view.setTextSize(size);
        view.setTextColor(Color.rgb(229, 235, 232));
        view.setPadding(dp(12), dp(5), dp(12), dp(5));
        return view;
    }
    private int dp(int value) { return Math.round(value * getResources().getDisplayMetrics().density); }
    private Button button(String label, int id, LinearLayout row, Runnable action) {
        Button button = new Button(this);
        button.setId(id);
        button.setText(label);
        button.setTextSize(12);
        button.setPadding(0, 0, 0, 0);
        button.setOnClickListener(v -> action.run());
        row.addView(button, new LinearLayout.LayoutParams(0, dp(48), 1));
        return button;
    }
    private void show(int selected) {
        index = Math.max(0, Math.min(FILES.length - 1, selected));
        title.setText((index + 1) + " / 3 · " + TITLES[index] + (landscape ? " · " + NOTES[index] : ""));
        note.setText(NOTES[index]);
        previous.setEnabled(index > 0);
        next.setEnabled(index < FILES.length - 1);
        imageLoaded = false;
        if (image.getScale() > 0) image.zoomBy(getResources().getDisplayMetrics().density / image.getScale());
        image.scrollTo(0, 0);
        image.setInitialScale(0);
        image.loadDataWithBaseURL("file:///android_asset/", "<!doctype html><html><head>"
            + "<meta name='viewport' content='width=device-width,initial-scale=1,minimum-scale=1,maximum-scale=8,user-scalable=yes'>"
            + "<style>html,body{margin:0;background:#141819}img{display:block;width:100vw;height:100vh;object-fit:contain;object-position:center top}</style></head><body>"
            + "<img src='" + FILES[index] + "' alt='" + TITLES[index] + " desktop concept'></body></html>", "text/html", "UTF-8", null);
    }
    @Override protected void onSaveInstanceState(Bundle saved) { super.onSaveInstanceState(saved); saved.putInt("concept", index); }
    @Override protected void onDestroy() { image.destroy(); super.onDestroy(); }
}
