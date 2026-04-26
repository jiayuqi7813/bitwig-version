# Bitwig Versions Extension (installable `.bwextension`)

This folder now contains a real Bitwig Java extension that can be compiled and packaged into a `.bwextension` file.

## 1) Build

You need Java 11+ and the Bitwig extension API jar.

```bash
cd apps/bitwig-extension
export BITWIG_API_JAR="/path/to/extension-api.jar"
./build.sh
```

Output file:

- `dist/BitwigVersions.bwextension`

## 2) Install in Bitwig

Copy the generated `.bwextension` into your Bitwig Controller Scripts/Extensions directory:

- **macOS**: `~/Documents/Bitwig Studio/Extensions`
- **Windows**: `%USERPROFILE%\Documents\Bitwig Studio\Extensions`
- **Linux**: `~/Bitwig Studio/Extensions`

Then restart Bitwig Studio and add the extension from **Settings → Controllers**.

## 3) Runtime behavior

- On init, extension checks `http://127.0.0.1:47321/health`.
- If your desktop app/API is not running, it shows popup:
  - `Bitwig Versions app is not running.`
- It also observes transport play state and triggers a quick-save request when playback starts.
