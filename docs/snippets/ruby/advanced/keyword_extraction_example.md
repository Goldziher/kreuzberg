```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::Config::Extraction.new(
  keywords: Kreuzberg::Config::Keywords.new(
    algorithm: Kreuzberg::KeywordAlgorithm::YAKE,
    max_keywords: 10,
    min_score: 0.3
  )
)

result = Kreuzberg.extract_file_sync('research_paper.pdf', config: config)

keywords = result.metadata&.dig('keywords') || []
keywords.each do |kw|
  text = kw['text']
  score = kw['score']
  puts "#{text}: #{score.round(3)}"
end
```
