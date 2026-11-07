# frozen_string_literal: true

begin
  require "kettle-soup-cover"

  if Kettle::Soup::Cover::DO_COV
    require "simplecov"
    require "kettle/soup/cover/config"

    SimpleCov.start
  end

rescue LoadError => error
  raise unless error.message.include?("kettle")
end

require "kettle/test/rspec"
require "version_gem/rspec"

RSpec.configure do |config|
  config.disable_monkey_patching!
  config.expect_with(:rspec) { |expectations| expectations.syntax = :expect }
end
