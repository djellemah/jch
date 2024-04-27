// NOTE jch is just what cxx-build wants to call it, because of the cargo package name.
#include "jch/src/wrapper.h"

// pull in the generated defintions from cxx.rs
// NOTE jch is just what cxx-build wants to call it, because of the cargo package name.
#include "jch/src/rapid.rs.h"

// TODO vendor or git submodule this into the build tree
#include "rapidjson/fwd.h"
#include "rapidjson/rapidjson.h"
#include "rapidjson/reader.h"

/*
  From rapidjson docs:

	rapidjson::Stream
    \brief Concept for reading and writing characters.

    For read-only stream, no need to implement PutBegin(), Put(), Flush() and PutEnd().
*/
/*
	This implements the Stream concept.

	We need it here, otherwise there's nowhere to hang the Ch typedef.
*/
class WrapRustStream {
public:
    typedef char Ch;

    WrapRustStream(RustStream & rust_stream) : _rust_stream(rust_stream) {}

    Ch Peek() const { return _rust_stream.Peek(); }
    Ch Take() { return _rust_stream.Take(); }
    size_t Tell() const { return _rust_stream.Tell(); } // 3

    // these can stay unimplemented apparently

    // Begin writing operation at the current read pointer.
    //! \return The begin writer pointer.
    // rapidjson::Reader interprets 0 as eof.
    Ch* PutBegin() { return reinterpret_cast<Ch*>(0); }
    void Put(Ch ch) { return _rust_stream.Put(ch); }
    void Flush() {  return _rust_stream.Flush(); }
    size_t PutEnd(Ch* chp) { return _rust_stream.PutEnd(chp); }

private:
    WrapRustStream(const WrapRustStream&);
    WrapRustStream& operator=(const WrapRustStream&);

    RustStream& _rust_stream;
};

// Implement this in c++ so it can instantiate the rapidjson templates.
void parse(RustHandler & handler, RustStream & incoming) {
	WrapRustStream stream(incoming);
  rapidjson::Reader().Parse(stream, handler);
}
