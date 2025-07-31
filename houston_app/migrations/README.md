This document assumes that your `$MONGODB_URI` includes the database name, as is required for `houston_app` itself.

# Database Migrations

This directory contains optional migrations for the database. To run them either:

## Use MongoDB Shell (`mongosh`)

The simplest way to run them is to use `mongosh`:

```sh
mongosh $MONGODB_URI $PATH_TO_MIGRATION
```

For example, to update a local `test` database, assuming the command is run in this directory:

```sh
mongosh mongodb://localhost/test ./2.15.1-starboard.messages.js
```

If everything went well, there should be no output.

Note that `mongosh` scripts are specified either as a path relative to the current working directory or as an absolute path.

## Use MongoDB Compass

Connect to your database server with the UI. Then, select the correct database and press "Open MongoDB shell".

In the new window/tab, paste the content of the script and press enter.

If everything went well, there should be no further output. You can verify the data is correct with the UI afterwards.

# Backups

It is recommended to back up your data before running these migrations.

To do so, use the `mongodump` utility:

```sh
mongodump --uri="$MONGODB_URI" --archive="$FILENAME"
```

To later restore the backup, use the `mongorestore` utility:

```sh
mongorestore --uri="$MONGODB_URI" --archive="$FILENAME"
```

Note that you cannot specify the database name here. The restored database name will match the original database name. If you intent to restore the database with a different name, use the [`--nsFrom` and `--nsTo`](https://www.mongodb.com/docs/database-tools/mongorestore/#std-option-mongorestore.--nsFrom) parameters.

For example, to back up your local `test` database to a file named `my-db-backup` and restore it later:

```sh
mongodump --uri="mongodb://localhost/test" --archive="my-db-backup"
mongorestore --uri="mongodb://localhost" --archive="my-db-backup"
```
