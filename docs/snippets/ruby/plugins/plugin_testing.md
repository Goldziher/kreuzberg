```ruby title="Ruby"
require 'kreuzberg'
require 'rspec'

describe 'Plugin Testing' do
  it 'registers and calls post-processor' do
    processor = ->(result) { result['metadata'] ||= {}; result }
    Kreuzberg.register_post_processor('test', processor)
    expect(Kreuzberg.list_post_processors).to include('test')
    Kreuzberg.unregister_post_processor('test')
  end

  it 'registers and validates' do
    validator = ->(result) do
      raise Kreuzberg::Errors::ValidationError, 'Too short' if result['content'].length < 10
    end
    Kreuzberg.register_validator('test-val', validator)
    expect(Kreuzberg.list_validators).to include('test-val')
    Kreuzberg.unregister_validator('test-val')
  end
end
```
