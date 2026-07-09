# SQLite Infrastructure

`src/infrastructure/sqlite` owns concrete SQLite schema, migration, and store
implementations.

Keep persistence mechanics, table schemas, migrations, and SQL-backed store
adapters here. Domain code owns store contracts and rules; application code
coordinates use cases over those contracts.
