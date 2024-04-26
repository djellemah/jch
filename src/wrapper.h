#pragma once

#include "rust/cxx.h"

// #include "../../src/rapidjson/include/rapidjson/fwd.h"
// #include "../../src/rapidjson/include/rapidjson/rapidjson.h"
// #include "../../src/rapidjson/include/rapidjson/reader.h"

namespace wut {
class RustStream;
class RustHandler;

class Local {
	const char * huh = "I dunno what go on";
};

void parse(RustHandler & handler, RustStream & incoming);
const char * hello();
}
