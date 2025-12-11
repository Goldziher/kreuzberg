```ruby
require 'net/http'
require 'uri'
require 'json'

# Single file extraction
uri = URI('http://localhost:8000/extract')
request = Net::HTTP::Post.new(uri)
form_data = [['files', File.open('document.pdf')]]
request.set_form form_data, 'multipart/form-data'

response = Net::HTTP.start(uri.hostname, uri.port) do |http|
  http.request(request)
end

results = JSON.parse(response.body)
puts results[0]['content']
```
