# jch

Parse really large json files, fast. Using a streaming parser so memory usage is limited.

For example:

- the schema for a 26Mb file is calculated in about 750ms using about 4Mb of RAM.

- the schema for a 432Mb file is calculated in about 20s using about 4Mb of RAM.

Currently only outputs the schema of a json file, in a non-standard format.

Is designed in a modular way so you can use it as a base for filtering json. Like `jq`.

Might grow some kind of path-filtering languages, like jsonpath or xpath.
