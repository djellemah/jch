#include "jch/wrapper.hpp"

// pull in the generated defintions from cxx.rs
#include "jch/src/rapid.rs.h"

/*
  From rapidjson docs:

	rapidjson::Stream
    \brief Concept for reading and writing characters.

    For read-only stream, no need to implement PutBegin(), Put(), Flush() and PutEnd().
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
    Ch* PutBegin() { return reinterpret_cast<Ch*>(0); }
    void Put(Ch ch) { return _rust_stream.Put(ch); }
    void Flush() {  return _rust_stream.Flush(); }
    size_t PutEnd(Ch* chp) { return _rust_stream.PutEnd(chp); }

private:
    WrapRustStream(const WrapRustStream&);
    WrapRustStream& operator=(const WrapRustStream&);

    RustStream& _rust_stream;
};

class WrapRustHandler {
public:
	typedef char Ch;
	WrapRustHandler(RustHandler & handler) : _rust_handler(handler) {}

	bool Null() { return _rust_handler.Null(); }
	bool Bool(bool b) { return _rust_handler.Bool(b); }
	bool Int(int i) { return _rust_handler.Int(i); }
	bool Uint(unsigned i) { return _rust_handler.Uint(i); }
	bool Int64(int64_t i) { return _rust_handler.Int64(i); }
	bool Uint64(uint64_t i) { return _rust_handler.Uint64(i); }
	bool Double(double d) { return _rust_handler.Double(d); }
	bool RawNumber(const char* str, size_t length, bool copy) { return _rust_handler.RawNumber(str, length, copy); }
	bool String(const char* str, size_t length, bool copy) { return _rust_handler.String(str, length, copy); }
	bool StartObject() { return _rust_handler.StartObject(); }
	bool Key(const char* str, size_t length, bool copy) { return _rust_handler.Key(str, length, copy); }
	bool EndObject(size_t memberCount) { return _rust_handler.EndObject(memberCount); }
	bool StartArray() { return _rust_handler.StartArray(); }
	bool EndArray(size_t elementCount) { return _rust_handler.EndArray(elementCount); }

private:
	RustHandler& _rust_handler;
};

void parse(RustHandler & handler, RustStream & incoming) {
	WrapRustStream stream(incoming);
  rapidjson::Reader().Parse(stream, handler);
}