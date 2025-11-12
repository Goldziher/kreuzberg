# Ruby Smoke App

Verifies that the `kreuzberg` gem installs and extracts a simple fixture.

## Run against RubyGems

```bash
cd e2e/smoke/ruby
bundle init --gemspec || true
bundle add kreuzberg
bundle exec ruby app.rb
```

## Run against a local gem

```bash
gem install path/to/kreuzberg-*.gem
cd e2e/smoke/ruby
ruby app.rb
```
