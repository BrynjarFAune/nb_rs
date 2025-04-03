NetBox Ingestion

An asynchronous library for collecting data, converting the data into netbox objects and pushing them to a netbox host.
Written in rust with tokio for async.

api keys, urls and such are required and should be defined in the src/config.toml file
simply copy the config_template.toml file and edit the variables to fit your system

FortiGate also requires the root certificate to be stored in certs/ in .crt format
