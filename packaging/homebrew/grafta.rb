# Rendered by scripts/homebrew-formula.sh on each release; the copy in
# n0ss4/homebrew-tap is the one `brew install n0ss4/tap/grafta` reads.
class Grafta < Formula
  desc "Figma's file format for your agent: read and write .fig files, offline"
  homepage "https://n0ss4.github.io/grafta"
  version "@VERSION@"
  license "MIT"

  base = "https://github.com/n0ss4/grafta/releases/download/v@VERSION@"

  on_macos do
    on_arm do
      url "#{base}/grafta-aarch64-apple-darwin.tar.gz"
      sha256 "@SHA_MACOS_ARM@"
    end
    on_intel do
      url "#{base}/grafta-x86_64-apple-darwin.tar.gz"
      sha256 "@SHA_MACOS_INTEL@"
    end
  end

  on_linux do
    on_arm do
      url "#{base}/grafta-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "@SHA_LINUX_ARM@"
    end
    on_intel do
      url "#{base}/grafta-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "@SHA_LINUX_INTEL@"
    end
  end

  def install
    bin.install "grafta"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/grafta --version")
  end
end
