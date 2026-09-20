# typed: false
# frozen_string_literal: true

class SuperStarTrek < Formula
  desc "Authentic modernized Super Star Trek with Bubbletea TUI and retro audio"
  homepage "https://github.com/scottdensmore/super-star-trek"
  version "2.0.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/scottdensmore/super-star-trek/releases/download/v#{version}/super-star-trek_#{version}_darwin_amd64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    if Hardware::CPU.arm?
      url "https://github.com/scottdensmore/super-star-trek/releases/download/v#{version}/super-star-trek_#{version}_darwin_arm64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/scottdensmore/super-star-trek/releases/download/v#{version}/super-star-trek_#{version}_linux_amd64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/scottdensmore/super-star-trek/releases/download/v#{version}/super-star-trek_#{version}_linux_arm64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "sst"
    doc.install "README.md", "c/sst.doc" if File.exist?("c/sst.doc")
  end

  test do
    assert_match "sst version", shell_output("#{bin}/sst --version")
  end
end
