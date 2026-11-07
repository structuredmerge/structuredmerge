# Coverage thresholds are configured through mise.toml and kettle-soup-cover.
SimpleCov.configure do
  if SimpleCov::Configuration.instance_methods.include?(:cover)
    cover "lib/**/*.rb", "lib/**/*.rake", "exe/*.rb"
  else
    track_files "{lib/**/*.rb,lib/**/*.rake,exe/*.rb}"
  end
end
