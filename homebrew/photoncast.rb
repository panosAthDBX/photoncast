# typed: true
# frozen_string_literal: true

cask "photoncast" do
  version "0.1.0-alpha"
  sha256 :no_check # Placeholder - must be updated with actual DMG SHA256

  url "https://github.com/photoncast/photoncast/releases/download/v#{version}/PhotonCast-#{version}.dmg"
  name "PhotonCast"
  desc "Fast, extensible macOS launcher"
  homepage "https://github.com/photoncast/photoncast"

  # Minimum macOS version as per spec
  depends_on macos: ">= :monterey"

  # Auto-updates via Sparkle framework (once implemented)
  auto_updates true

  app "PhotonCast.app"

  # Uninstall launchctl services if any are added in the future
  # uninstall launchctl: "com.photoncast.app.helper"

  zap trash: [
    "~/Library/Application Support/PhotonCast",
    "~/Library/Preferences/com.photoncast.app.plist",
    "~/Library/Caches/com.photoncast.app",
    "~/Library/Logs/PhotonCast",
    "~/Library/Saved Application State/com.photoncast.app.savedState",
  ]

  caveats <<~EOS
    PhotonCast requires accessibility permissions to function properly.
    You may be prompted to grant these permissions on first launch.

    To enable PhotonCast to appear in the Dock:
    1. Open Preferences (⌘,)
    2. Enable "Show in Dock"
    3. Restart the application
  EOS
end
