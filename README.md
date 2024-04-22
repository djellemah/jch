# jch

Parse really large json files, fast. Using a streaming parser so memory usage is limited.

For example:

- the schema for a 26Mb file is calculated in about 750ms using about 4Mb of RAM.

- the schema for a 432Mb file is calculated in about 20s using about 4Mb of RAM.

Currently only outputs the schema of a json file, in a non-standard format.

Is designed in a modular way so you can use it as a base for filtering json. Somewhat like `jq`.

Might grow into some kind of path-filtering language, like jsonpath or xpath.

# Download a release

See releases on the right of this repo page. Executables for linux, windows, macos.

# How to build

If there's no executable to suit you, you'll need to build your own:

Clone this repo.

``` bash
cargo build --release
```
# Usage example

``` bash
curl \
https://raw.githubusercontent.com/json-iterator/test-data/master/large-file.json \
| target/release/jch -s
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
