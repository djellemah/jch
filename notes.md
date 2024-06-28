# lots of jq implementations
https://github.com/fiatjaf/awesome-jq

# zerocopy
https://crates.io/crates/zerocopy

# SIMD json
https://github.com/simdjson/simdjson

and a discussion of whether it works with streaming
https://github.com/simdjson/simdjson/issues/1361

BUT their context-blindness issue can be solved with a path. Like I'm doing.

And one could hash the path if a flat data-address-space was preferred.

# other streaming parsers
json-stream by alexmaco is mostly dead

Struson seems alive, but uses a fn for parsing elements of arrays, so would need a coroutine (or something) to invert that.
https://github.com/marcono1234/struson
https://crates.io/crates/struson

cargo-make cos its nice
https://github.com/sagiegurari/cargo-make

rapidjson in SAX mode
https://rapidjson.org/md_doc_sax.html

wrapper for simdjson. Does not have a streaming mode.
simdjson-rust = "0.3.0-alpha.2"

fast, but does tapes, ie pointers into the data
which seems to be the main approach of simdjson
https://github.com/datalust/squirrel-json?ref=blog.datalust.co

Maybe serde with this?
https://stackoverflow.com/questions/75936264/how-can-i-build-up-a-stateful-streaming-parser-with-serde-json

# Large sample json files
25M https://raw.githubusercontent.com/json-iterator/test-data/master/large-file.json

181M https://github.com/zemirco/sf-city-lots-json/blob/master/citylots.json

Some seriously big data here, finally. But take a wild guess which ones are large.

https://catalog.data.gov/dataset/?q=large&sort=views_recent+desc&res_format=JSON&ext_location=&ext_bbox=&ext_prev_extent=

This one seems to be large:
https://data.cdc.gov/api/views/qnzd-25i4/rows.json?accessType=DOWNLOAD

Large json datasets here, but not really helpful because it's records, each containing some json.
https://openlibrary.org/developers/dumps

some more test files here
https://transparency-in-coverage.uhc.com/

But it's pretty horrible.
