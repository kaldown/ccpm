class Ccpm < Formula
  desc "Claude Code Plugin Manager - TUI for managing Claude Code plugins"
  homepage "https://github.com/ccpm/ccpm"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/ccpm/ccpm/releases/download/v#{version}/ccpm-macos-arm64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM64"
    end
    on_intel do
      url "https://github.com/ccpm/ccpm/releases/download/v#{version}/ccpm-macos-x86_64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/ccpm/ccpm/releases/download/v#{version}/ccpm-linux-arm64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    end
    on_intel do
      url "https://github.com/ccpm/ccpm/releases/download/v#{version}/ccpm-linux-x86_64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
    end
  end

  def install
    bin.install "ccpm"
  end

  test do
    assert_match "Claude Code Plugin Manager", shell_output("#{bin}/ccpm --help")
  end
end
