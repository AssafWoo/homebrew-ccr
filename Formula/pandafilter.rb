class Pandafilter < Formula
  desc "LLM token optimizer for Claude Code — 60-90% token savings on dev operations"
  homepage "https://github.com/AssafWoo/PandaFilter"
  license "MIT"
  version "1.0.5"

  depends_on "jq"

  # Prebuilt binaries — no Rust/LLVM build dependencies, installs in seconds.
  # Each tarball contains the panda binary + libonnxruntime dylib bundled together.
  on_arm do
    url "https://github.com/AssafWoo/PandaFilter/releases/download/v1.0.5/panda-macos-arm64.tar.gz"
    sha256 "011286d940d61e2ac478ef349ad387a34078b928cd40f3bb4a8ce594d2c8c692"
  end

  on_intel do
    url "https://github.com/AssafWoo/PandaFilter/releases/download/v1.0.5/panda-macos-x86_64.tar.gz"
    sha256 "6a56dd0bf909ca57e2b4b102a76efe27ea0370dd042a601fa68f7b807555c677"
  end

  def install
    bin.install "panda"
    # Install the bundled ORT dylib and fix rpath so the binary finds it
    dylib = Dir["libonnxruntime*.dylib"].first
    if dylib
      lib.install dylib
      system "install_name_tool", "-add_rpath", lib.to_s, "#{bin}/panda"
    end

    # Compatibility shim bundled in the tarball — install it so old `ccr` hooks keep working
    bin.install "ccr"
  end

  def post_install
    # Register hooks (fast, no network). BERT model downloads lazily on first use.
    # quiet_system won't fail the install, but we check the result to guide the user.
    claude_ok = quiet_system bin/"panda", "init", "--skip-model"
    cursor_ok = quiet_system bin/"panda", "init", "--agent", "cursor", "--skip-model"

    if claude_ok || cursor_ok
      ohai "Hooks installed. Run `panda doctor` to verify."
    else
      opoo "Hook setup could not complete automatically."
      puts "  Run manually after install:"
      puts "    panda init"
      puts "    panda doctor"
    end
  end

  def caveats
    <<~EOS
      Verify your installation:
        panda doctor

      If doctor reports issues, re-run setup:
        panda init                      # Claude Code
        panda init --agent cursor       # Cursor

      Then restart your coding agent for hooks to take effect.
    EOS
  end

  test do
    assert_match "filter", shell_output("#{bin}/panda --help")
    assert_match(/\S/, pipe_output("#{bin}/panda filter", "hello world\n"))
  end
end
