class Ccr < Formula
  desc "LLM token optimizer for Claude Code — 60-90% token savings on dev operations"
  homepage "https://github.com/AssafWoo/homebrew-ccr"
  url "https://github.com/AssafWoo/homebrew-ccr/archive/refs/tags/v0.5.15.tar.gz"
  sha256 "PLACEHOLDER_RELEASE_SHA256" # updated automatically by release CI
  license "MIT"
  head "https://github.com/AssafWoo/homebrew-ccr.git", branch: "main"

  depends_on "rust" => :build

  # The `ort` crate (ONNX Runtime Rust bindings) downloads a pre-built ORT
  # binary during `cargo build`. Homebrew's sandbox blocks network access at
  # build time, so we fetch ORT here as a resource and point `ort` at it via
  # ORT_LIB_LOCATION before building.
  #
  # ORT version: 1.20.1 (required by ort = "2.0.0-rc.9")
  # To verify SHA256: shasum -a 256 <downloaded-tgz>
  on_arm do
    resource "ort-runtime" do
      url "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-osx-arm64-1.20.1.tgz"
      sha256 "PLACEHOLDER_ORT_ARM64_SHA256"
    end
  end

  on_intel do
    resource "ort-runtime" do
      url "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-osx-x86_64-1.20.1.tgz"
      sha256 "PLACEHOLDER_ORT_X86_64_SHA256"
    end
  end

  def install
    # Extract ORT and expose it to the ort-sys build script
    ort_dir = buildpath/"ort-extracted"
    ort_dir.mkpath
    resource("ort-runtime").stage(ort_dir)

    # ort-sys looks for the dylib in ORT_LIB_LOCATION
    # The tarball unpacks to onnxruntime-osx-{arch}-{ver}/lib/
    ort_lib = Dir["#{ort_dir}/**/lib"].first
    ENV["ORT_LIB_LOCATION"] = ort_lib if ort_lib

    # Prevent ort-sys from attempting a network download
    ENV["ORT_STRATEGY"] = "system"

    system "cargo", "install", *std_cargo_args(path: "ccr")
  end

  def caveats
    <<~EOS
      Register CCR with Claude Code:
        ccr init
    EOS
  end

  test do
    # Verify the binary runs and core subcommands are present
    assert_match "filter", shell_output("#{bin}/ccr --help")
    # filter subcommand must compress stdin
    assert_match(/\S/, pipe_output("#{bin}/ccr filter", "hello world\n"))
  end
end
