#pragma once

#include "rapidjson/fwd.h"
#include "rapidjson/rapidjson.h"
#include "rapidjson/reader.h"

/*
// apparently this implements the Handler concept
struct RustHandler {
    bool Null();
    bool Bool(bool b);
    bool Int(int i);
    bool Uint(unsigned i);
    bool Int64(int64_t i);
    bool Uint64(uint64_t i);
    bool Double(double d);
    bool RawNumber(const char* str, size_t length, bool copy);
    bool String(const char* str, size_t length, bool copy);
    bool StartObject();
    bool Key(const char* str, size_t length, bool copy);
    bool EndObject(size_t memberCount);
    bool StartArray();
    bool EndArray(size_t elementCount);
};

struct RustStream  {
    typedef char Ch;

    char Peek() const { return *src_; }
    char Take() { return *src_++; }
    size_t Tell() const { return static_cast<size_t>(src_ - head_); }

    char* PutBegin() { RAPIDJSON_ASSERT(false); return 0; }
    void Put(char) { RAPIDJSON_ASSERT(false); }
    void Flush() { RAPIDJSON_ASSERT(false); }
    size_t PutEnd(char*) { RAPIDJSON_ASSERT(false); return 0; }

    const char* src_;     //!< Current read position.
    const char* head_;    //!< Original head of the string.
};
*/

struct RustStream;
struct RustHandler;

void parse(RustHandler & handler, RustStream & incoming);
