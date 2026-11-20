# Install Chill Center as a web app

[Français](PWA-FR.md) | **English**

Open the client on the device to install, on the same network as the Pi.
The usual HTTP URL (`http://<Pi-address>`, configured port, often 80) remains a usable website.
For an installable PWA and screen wake lock, use the existing HTTPS endpoint:
`https://<Pi-address>:<HTTPS-port>` (8443 in the tested installation). The browser must trust the server certificate.
A page-only certificate exception may not allow service workers; verify the
certificate and establish trust on the device. Do not disable TLS verification.

- Android Chrome: **Install app** in Chill Center, then the native prompt.
  If the prompt is not available yet, use ⋮ → **Install app**.
- iPhone/iPad Safari: **Share → Add to Home Screen**.
- Mac Chrome/Vivaldi: **Install app** or the browser’s site installation menu.
- Mac Safari: **File → Add to Dock**.

The app opens in its own window. The Pi and videos remain required: movies,
API responses and watch history are never stored by the offline shell cache.
HTTP and HTTPS have separate browser storage; select your profile on the new origin.

After an update, reopen/reload the app to load the new client. An update never
automatically reloads an ongoing video. A PWA uses the browser/OS codecs and
native player; installation does not provide a missing video codec. The install
button shows guidance if installation or certificate trust is unavailable.

Current tests cover macOS Chromium and synthetic mobile fixtures. Physical
Safari iPhone and Chrome Pixel validation is still required.
