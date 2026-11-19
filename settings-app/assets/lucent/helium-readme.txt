YASB Accent: a Chromium theme written by YASB Settings (the Lucent theme).

Load it once:
  1. Open helium://extensions (chrome://extensions in other browsers)
  2. Turn on Developer mode
  3. Load unpacked, and pick this folder

When the accent changes, YASB Settings rewrites manifest.json and deletes the 'Cached Theme.pak' file in this folder (the browser keeps
a built copy of the theme there). The browser reads a changed theme when it starts: restart it to see the new colours.
If a restart still shows the old colours, remove the theme and Load unpacked again.
