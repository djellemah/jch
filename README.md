# jch

Parse really large json files, fast. Using a streaming parser so memory usage is limited.

For example:

- the schema for a 26Mb file is calculated in about 750ms using about 4Mb of RAM.

- the schema for a 432Mb file is calculated in about 20s using about 4Mb of RAM.

Currently only outputs the schema of a json file, in a non-standard format.

Is designed in a modular way so you can use it as a base for filtering json. Somewhat like `jq`, but you write your filtering code in Rust. See 'Design' below for a description.

Might grow some kind of path-filtering language, like jsonpath or xpath.

# Download a release

See releases on the right of this repo page. Executables for linux, windows, macos.

1. Download it
1. rename it to something convenient (`jch` is recommended)
1. if you're on a unixy OS, say `chmod a+x jch`
1. copy it to a `bin` directory on your `PATH` somewhere. On a unixy OS `~/bin` often works, likewise `~/.local/bin`

# How to build

If there's no executable to suit you, you'll need to build your own:

Clone this repo:
```
git clone --recurse-submodules --shallow-submodules https://github.com/djellemah/jch.git
```

Build it:
``` bash
cargo build --release
```
# Usage example

``` bash
curl \
https://raw.githubusercontent.com/json-iterator/test-data/master/large-file.json \
| jch -s
```

will output

```
String(47):11351                    []/actor/avatar_url
String(0):11351                     []/actor/gravatar_id
Number(Unsigned(665991)):11351      []/actor/id
String(7):11351                     []/actor/login
String(36):11351                    []/actor/url
String(20):11351                    []/created_at
String(10):11351                    []/id
String(48):3245                     []/org/avatar_url
String(0):3245                      []/org/gravatar_id
Number(Unsigned(9285252)):3245      []/org/id
String(11):3245                     []/org/login
String(39):3245                     []/org/url
String(7):3314                      []/payload/action
…
…
…
Number(Unsigned(818676)):60         []/payload/release/id
[String(0):59,Null:1]               []/payload/release/name
Boolean:60                          []/payload/release/prerelease
String(20):60                       []/payload/release/published_at
String(6):60                        []/payload/release/tag_name
String(63):60                       []/payload/release/tarball_url
String(6):60                        []/payload/release/target_commitish
String(82):60                       []/payload/release/upload_url
String(64):60                       []/payload/release/url
String(63):60                       []/payload/release/zipball_url
Number(Unsigned(1)):5815            []/payload/size
Boolean:11351                       []/public
Number(Unsigned(28688495)):11351    []/repo/id
String(13):11351                    []/repo/name
String(42):11351                    []/repo/url
String(11):11351                    []/type
```

## Interpreting the output

The fundamental perspective here is that a tree is a map of paths to values, and the schema of a tree is a map from the paths to the types of those values.

Right hand column is the path, excluding numeric indexes (ie arrays). You'll still see `[]` where arrays would be.

Left hand column is the aggregated type of all leaves at the path in the right-hand column. That is, for the corresponding path on the right, the value had these characteristics:

- `String(n)` means the largest string encountered has length `n`

- `Unsigned(n)` means n was the largest value encountered

- `Signed(min,max)` means min and max encountered

- `Float(min,max)` means min and max encountered

- `Xxx:nnnn` means `nnnn` was the number of values encountered, ie the number of leaf nodes matching the path.

- If more than one type was encountered at the path, the left hand column will contain an array of characteristics, as above. That is, it's a sum type.

# Advanced Build
You can use an existing rapidjson tree by specifying the `RAPIDJSON_INCLUDE` env var.

Either on the command line

```
RAPIDJSON_INCLUDE=your_source_dir cargo build
```

or in `.config/cargo.toml` like this

```
[env]
RAPIDJSON_INCLUDE = "rapidjson/include"
```

# 1000 words
scroll right >>>
```
                                                                            +------------+
+----------+   +----------+                                             +->-| FnSnd      |
|RapidJson |->-|RingBuffer|--->--+                                      |   +------------+
+----------+   +----------+      |                                      |                
                             +--------+     +---------+     +--------+  |   +------------+
                             | Parser |-->--| Handler |-->--| Sender |--+->-| Channel    |
                             +--------+     +---------+     +--------+  |   +------------+
      +-----------------+        |                                      |                 
      |json-event-parser|--->----+                                      |   +------------+
      +-----------------+                                               +->-| Schema     |
                                                                        |   +------------+
                                                                        |                 
                                                                        |   +------------+
                                                                        +->-| RingBuffer |
                                                                            +------------+
```
scroll right >>>
# Design

This is for the people who read this far, and feel like reading some more. I flatter myself and fondly imagine that you like my prose style.

The fundamental underlying idea here is that a tree can be viewed as a map of paths to leafs, where the more common view is a set of nodes with children. Whatever receives the events from the streaming parser has the responsibility to track changes to the path as they emerges from the event stream. Having the full path for every event addresses what simdjson calls "context blindness".

There are currently 2 backend parsers - the `json_event_parser` crate and the `c++` templates `rapidjson`. Performance is remarkably similar, probably because rapidjson gets slowed down by calling the rust interop functions.

`json_event_parser` is a pull api, so events are extracted directly and sent to the handler. `rapidjson` is a push api, so a lock-free ringbuffer is used to send events to the handler.

To avoid busy-waiting on the ringbuffer, that code path relies on `std::thread::park`. Those seem to not require a mutex. I'm hoping they make use of `pthread`'s signal handling to achieve thread wakeup.

Both of those backend parsers can also use `crossbeam::channel`. Which, in the highly rigorous eyeball-performance tests I've conducted, is not really slower than the ringbuffer. Also, `crossbeam::channel` is noticeably faster than `std::sync::mpsc::channel`. And slightly less pernickety to use in the code.

Anyways, the handler keeps track of the path using `rpds::Vector` whose persitent-ness works well here since the path prefixes quite often change relatively slowly especially at the top level. The handler converts the events (which have `ref`s to the parser's internal buffers) into events that can be distributed to, well whatever other things know how to receive `(path,leaf)` from the handler. There are a few of those sprinkled around the code: one produces the schema output above; the other writes the packets to MessagePack with the intention of implementing the shredder algorithm from the dremel paper one day. Another one just converts the json events back into proper json using `serde_json`.

Filtering can also happen in the handler, where a predicate method allows the receiver of the events to discard events based on their path. That part of the design hasn't found its proper home yet.
