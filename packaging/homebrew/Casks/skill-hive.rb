cask "skill-hive" do
  version "1.0.0"

  name "Skill Hive"
  desc "Create, generate, and test agent skills"
  homepage "https://github.com/crzyc0d3r/skill-hive"

  on_arm do
    url "https://github.com/crzyc0d3r/skill-hive/releases/download/v#{version}/Skill.Hive_#{version}_aarch64.dmg"
    sha256 "PENDING"
  end
  on_intel do
    url "https://github.com/crzyc0d3r/skill-hive/releases/download/v#{version}/Skill.Hive_#{version}_x64.dmg"
    sha256 "PENDING"
  end

  app "Skill Hive.app"
end
