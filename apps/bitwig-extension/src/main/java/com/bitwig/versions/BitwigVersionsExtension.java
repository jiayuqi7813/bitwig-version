package com.bitwig.versions;

import com.bitwig.extension.controller.ControllerExtension;
import com.bitwig.extension.controller.ControllerExtensionDefinition;
import com.bitwig.extension.controller.api.ControllerHost;
import com.bitwig.extension.controller.api.Transport;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;

public class BitwigVersionsExtension extends ControllerExtension {
    private static final String BASE = "http://127.0.0.1:47321";
    private final HttpClient http = HttpClient.newBuilder()
            .connectTimeout(Duration.ofSeconds(3))
            .build();

    private Transport transport;

    protected BitwigVersionsExtension(
            final ControllerExtensionDefinition definition,
            final ControllerHost host
    ) {
        super(definition, host);
    }

    @Override
    public void init() {
        transport = getHost().createTransport();
        getHost().println("Bitwig Versions extension initialized.");

        // Quick health check at startup.
        if (get("/health") == null) {
            notifyNotRunning();
        }

        // Trigger a quick-save when transport starts.
        transport.isPlaying().addValueObserver(isPlaying -> {
            if (isPlaying) {
                saveVersion();
            }
        });
    }

    @Override
    public void flush() {
        // no-op
    }

    @Override
    public void exit() {
        getHost().println("Bitwig Versions extension exited.");
    }

    public void openApp() {
        if (health()) {
            getHost().println("Bitwig Versions app is running.");
        } else {
            notifyNotRunning();
        }
    }

    public void saveVersion() {
        postJson("/snapshots/save", "{\"message\":\"Quick save from Bitwig\",\"author\":\"Bitwig Extension\"}");
    }

    public void push() {
        postJson("/git/push", "{}");
    }

    public void pull() {
        postJson("/git/pull", "{}");
    }

    public void status() {
        get("/status");
    }

    private boolean health() {
        return get("/health") != null;
    }

    private String get(String endpoint) {
        HttpRequest req = HttpRequest.newBuilder(URI.create(BASE + endpoint))
                .GET()
                .timeout(Duration.ofSeconds(3))
                .build();
        try {
            HttpResponse<String> response = http.send(req, HttpResponse.BodyHandlers.ofString());
            return response.body();
        } catch (IOException | InterruptedException e) {
            notifyNotRunning();
            return null;
        }
    }

    private String postJson(String endpoint, String json) {
        HttpRequest req = HttpRequest.newBuilder(URI.create(BASE + endpoint))
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .header("Content-Type", "application/json")
                .timeout(Duration.ofSeconds(3))
                .build();
        try {
            HttpResponse<String> response = http.send(req, HttpResponse.BodyHandlers.ofString());
            return response.body();
        } catch (IOException | InterruptedException e) {
            notifyNotRunning();
            return null;
        }
    }

    private void notifyNotRunning() {
        getHost().showPopupNotification("Bitwig Versions app is not running.");
    }
}
