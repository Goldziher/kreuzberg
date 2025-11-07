# frozen_string_literal: true

require 'bundler/setup'
require 'rspec'
require_relative 'helpers'

RSpec.configure do |config|
  config.order = :defined
end
