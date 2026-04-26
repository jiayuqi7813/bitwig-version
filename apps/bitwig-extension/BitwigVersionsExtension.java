import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;

public class BitwigVersionsExtension {
    private static final String BASE = "http://127.0.0.1:47321";
    private final HttpClient http = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(3)).build();

    public void openApp() {
        if (health()) {
            // Bitwig Desktop API hook should open window here.
            System.out.println("Bitwig Versions app is running.");
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
        System.out.println("Bitwig Versions app is not running.");
    }
}
