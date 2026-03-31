class Ccr < Formula
  desc "LLM token optimizer for Claude Code — 60-90% token savings on dev operations"
  homepage "https://github.com/AssafWoo/homebrew-ccr"
  license "MIT"

  # Prebuilt binaries — no Rust/LLVM build dependencies, installs in seconds.
  # Each tarball contains the ccr binary + libonnxruntime dylib bundled together.
  on_arm do
    url "https://github.com/AssafWoo/homebrew-ccr/releases/download/v0.5.20/ccr-macos-arm64.tar.gz"
    sha256 "7a30cab3bf33f54d00c0cb8846d1a90b1f9f90e7367266d14e3c19dba5b4569d"
  end

  on_intel do
    url "https://github.com/AssafWoo/homebrew-ccr/releases/download/v0.5.20/ccr-macos-x86_64.tar.gz"
    sha256 "90b0a647d83a922aadef69879185ac4456bb6d4be8232f3240c8d16a9d26d43b"
  end

  def install
    bin.install "ccr"
    # Install the bundled ORT dylib and fix rpath so the binary finds it
    dylib = Dir["libonnxruntime*.dylib"].first
    if dylib
      lib.install dylib
      system "install_name_tool", "-add_rpath", lib.to_s, "#{bin}/ccr"
    end
  end

  def post_install
    # Pre-download the BERT model and register Claude Code hooks automatically.
    # Runs as the installing user so ~/.cache and ~/.claude are correct.
    # quiet_system — don't fail the install if Claude Code isn't set up yet.
    quiet_system bin/"ccr", "init"
  end

  def caveats
    <<~EOS
      CCR setup runs automatically during install (hooks + BERT model download).
      If you see Claude Code hook errors, run manually:
        ccr init
    EOS
  end

  test do
    assert_match "filter", shell_output("#{bin}/ccr --help")
    assert_match(/\S/, pipe_output("#{bin}/ccr filter", "hello world\n"))
  end
end
