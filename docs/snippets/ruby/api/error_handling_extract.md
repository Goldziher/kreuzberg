```ruby title="Ruby"
require 'net/http'
require 'uri'
require 'json'

begin
  uri = URI('http://localhost:8000/extract')
  request = Net::HTTP::Post.new(uri)

  response = Net::HTTP.start(uri.hostname, uri.port) do |http|
    http.request(request)
  end

  if response.code.to_i >= 400
    error = JSON.parse(response.body)
    puts "Error: #{error['error_type']}: #{error['message']}"
  else
    results = JSON.parse(response.body)
    # Process results
  end
rescue => e
  puts "Request failed: #{e.message}"
end
```
