class SkillHiveCli < Formula
  desc "Skill Hive command line"
  homepage "https://github.com/crzyc0d3r/skill-hive"
  version "1.0.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/crzyc0d3r/skill-hive/releases/download/v#{version}/skill-hive-cli-aarch64-apple-darwin"
      sha256 "PENDING"
    end
    on_intel do
      url "https://github.com/crzyc0d3r/skill-hive/releases/download/v#{version}/skill-hive-cli-x86_64-apple-darwin"
      sha256 "PENDING"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/crzyc0d3r/skill-hive/releases/download/v#{version}/skill-hive-cli-aarch64-unknown-linux-gnu"
      sha256 "PENDING"
    end
    on_intel do
      url "https://github.com/crzyc0d3r/skill-hive/releases/download/v#{version}/skill-hive-cli-x86_64-unknown-linux-gnu"
      sha256 "PENDING"
    end
  end

  def install
    bin.install Dir["skill-hive-cli*"].first => "skill-hive-cli"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/skill-hive-cli --version")
  end
end
