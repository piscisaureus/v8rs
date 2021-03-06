#include <cassert>
#include <cstdint>
#include <iostream>

#include "support.h"
#include "v8/include/libplatform/libplatform.h"
#include "v8/include/v8-inspector.h"
#include "v8/include/v8-platform.h"
#include "v8/include/v8.h"

using namespace support;

static_assert(sizeof(v8::ScriptOrigin) == sizeof(size_t) * 7,
              "ScriptOrigin size mismatch");

static_assert(sizeof(v8::HandleScope) == sizeof(size_t) * 3,
              "HandleScope size mismatch");

static_assert(sizeof(v8::PromiseRejectMessage) == sizeof(size_t) * 3,
              "PromiseRejectMessage size mismatch");

static_assert(sizeof(v8::Locker) == sizeof(size_t) * 2, "Locker size mismatch");

static_assert(sizeof(v8::ScriptCompiler::Source) == sizeof(size_t) * 8,
              "Source size mismatch");

static_assert(sizeof(v8::ReturnValue<v8::Value>) == sizeof(size_t) * 1,
              "ReturnValue size mismatch");

static_assert(sizeof(v8::TryCatch) == sizeof(size_t) * 6,
              "TryCatch size mismatch");

extern "C" {

void v8__V8__SetFlagsFromCommandLine(int* argc, char** argv) {
  v8::V8::SetFlagsFromCommandLine(argc, argv, true);
}

const char* v8__V8__GetVersion() { return v8::V8::GetVersion(); }

void v8__V8__InitializePlatform(v8::Platform& platform) {
  v8::V8::InitializePlatform(&platform);
}

void v8__V8__Initialize() { v8::V8::Initialize(); }

bool v8__V8__Dispose() { return v8::V8::Dispose(); }

void v8__V8__ShutdownPlatform() { v8::V8::ShutdownPlatform(); }

// This function consumes the Isolate::CreateParams object. The Isolate takes
// ownership of the ArrayBuffer::Allocator referenced by the params object.
v8::Isolate* v8__Isolate__New(v8::Isolate::CreateParams& params) {
  auto isolate = v8::Isolate::New(params);
  delete &params;
  return isolate;
}

void v8__Isolate__Dispose(v8::Isolate* isolate) {
  auto allocator = isolate->GetArrayBufferAllocator();
  isolate->Dispose();
  delete allocator;
}

void v8__Isolate__Enter(v8::Isolate* isolate) { isolate->Enter(); }

void v8__Isolate__Exit(v8::Isolate* isolate) { isolate->Exit(); }

void v8__Isolate__SetPromiseRejectCallback(v8::Isolate* isolate,
                                           v8::PromiseRejectCallback callback) {
  isolate->SetPromiseRejectCallback(callback);
}

void v8__Isolate__SetCaptureStackTraceForUncaughtExceptions(
    v8::Isolate* isolate, bool capture, int frame_limit) {
  // Note: StackTraceOptions are deprecated so we don't bother to bind to it.
  isolate->SetCaptureStackTraceForUncaughtExceptions(capture, frame_limit);
}

bool v8__Isolate__AddMessageListener(v8::Isolate& isolate,
                                     v8::MessageCallback callback) {
  return isolate.AddMessageListener(callback);
}

v8::Isolate::CreateParams* v8__Isolate__CreateParams__NEW() {
  return new v8::Isolate::CreateParams();
}

// This function is only called if the Isolate::CreateParams object is *not*
// consumed by Isolate::New().
void v8__Isolate__CreateParams__DELETE(v8::Isolate::CreateParams& self) {
  delete self.array_buffer_allocator;
  delete &self;
}

// This function takes ownership of the ArrayBuffer::Allocator.
void v8__Isolate__CreateParams__SET__array_buffer_allocator(
    v8::Isolate::CreateParams& self, v8::ArrayBuffer::Allocator* value) {
  delete self.array_buffer_allocator;
  self.array_buffer_allocator = value;
}

void v8__HandleScope__CONSTRUCT(uninit_t<v8::HandleScope>& buf,
                                v8::Isolate* isolate) {
  construct_in_place<v8::HandleScope>(buf, isolate);
}

void v8__HandleScope__DESTRUCT(v8::HandleScope& self) { self.~HandleScope(); }

v8::Isolate* v8__HandleScope__GetIsolate(const v8::HandleScope& self) {
  return self.GetIsolate();
}

void v8__Locker__CONSTRUCT(uninit_t<v8::Locker>& buf, v8::Isolate* isolate) {
  construct_in_place<v8::Locker>(buf, isolate);
}

void v8__Locker__DESTRUCT(v8::Locker& self) { self.~Locker(); }

void v8__ScriptCompiler__Source__CONSTRUCT(
    uninit_t<v8::ScriptCompiler::Source>& buf, v8::String* source_string,
    v8::ScriptOrigin& origin) {
  construct_in_place<v8::ScriptCompiler::Source>(
      buf, ptr_to_local(source_string), origin);
}

void v8__ScriptCompiler__Source__DESTRUCT(v8::ScriptCompiler::Source& self) {
  self.~Source();
}

v8::Module* v8__ScriptCompiler__CompileModule(
    v8::Isolate* isolate, v8::ScriptCompiler::Source* source,
    v8::ScriptCompiler::CompileOptions options,
    v8::ScriptCompiler::NoCacheReason no_cache_reason) {
  v8::MaybeLocal<v8::Module> maybe_local = v8::ScriptCompiler::CompileModule(
      isolate, source, options, no_cache_reason);
  if (maybe_local.IsEmpty()) {
    return nullptr;
  } else {
    return local_to_ptr(maybe_local.ToLocalChecked());
  }
}

bool v8__Value__IsUndefined(const v8::Value& self) {
  return self.IsUndefined();
}

bool v8__Value__IsNull(const v8::Value& self) { return self.IsNull(); }

bool v8__Value__IsNullOrUndefined(const v8::Value& self) {
  return self.IsNullOrUndefined();
}

v8::Primitive* v8__Null(v8::Isolate* isolate) {
  return local_to_ptr(v8::Null(isolate));
}

v8::Primitive* v8__Undefined(v8::Isolate* isolate) {
  return local_to_ptr(v8::Undefined(isolate));
}

v8::Boolean* v8__True(v8::Isolate* isolate) {
  return local_to_ptr(v8::True(isolate));
}

v8::Boolean* v8__False(v8::Isolate* isolate) {
  return local_to_ptr(v8::False(isolate));
}

v8::String* v8__String__NewFromUtf8(v8::Isolate* isolate, const char* data,
                                    v8::NewStringType type, int length) {
  return maybe_local_to_ptr(
      v8::String::NewFromUtf8(isolate, data, type, length));
}

int v8__String__Length(const v8::String& self) { return self.Length(); }

int v8__String__Utf8Length(const v8::String& self, v8::Isolate* isolate) {
  return self.Utf8Length(isolate);
}

int v8__String__WriteUtf8(const v8::String& self, v8::Isolate* isolate,
                          char* buffer, int length, int* nchars_ref,
                          int options) {
  return self.WriteUtf8(isolate, buffer, length, nchars_ref, options);
}

v8::Object* v8__Object__New(v8::Isolate* isolate,
                            v8::Local<v8::Value> prototype_or_null,
                            v8::Local<v8::Name>* names,
                            v8::Local<v8::Value>* values, size_t length) {
  return local_to_ptr(
      v8::Object::New(isolate, prototype_or_null, names, values, length));
}

v8::Isolate* v8__Object__GetIsolate(v8::Object& self) {
  return self.GetIsolate();
}

v8::Number* v8__Number__New(v8::Isolate* isolate, double value) {
  return *v8::Number::New(isolate, value);
}

double v8__Number__Value(const v8::Number& self) { return self.Value(); }

v8::Integer* v8__Integer__New(v8::Isolate* isolate, int32_t value) {
  return *v8::Integer::New(isolate, value);
}

v8::Integer* v8__Integer__NewFromUnsigned(v8::Isolate* isolate,
                                          uint32_t value) {
  return *v8::Integer::NewFromUnsigned(isolate, value);
}

int64_t v8__Integer__Value(const v8::Integer& self) { return self.Value(); }

v8::ArrayBuffer::Allocator* v8__ArrayBuffer__Allocator__NewDefaultAllocator() {
  return v8::ArrayBuffer::Allocator::NewDefaultAllocator();
}

void v8__ArrayBuffer__Allocator__DELETE(v8::ArrayBuffer::Allocator& self) {
  delete &self;
}

v8::Context* v8__Context__New(v8::Isolate* isolate) {
  // TODO: optional arguments.
  return *v8::Context::New(isolate);
}

void v8__Context__Enter(v8::Context& self) { self.Enter(); }

void v8__Context__Exit(v8::Context& self) { self.Exit(); }

v8::Isolate* v8__Context__GetIsolate(v8::Context& self) {
  return self.GetIsolate();
}

v8::Object* v8__Context__Global(v8::Context& self) { return *self.Global(); }

v8::String* v8__Message__Get(const v8::Message* self) {
  return local_to_ptr(self->Get());
}

v8::Isolate* v8__Message__GetIsolate(const v8::Message* self) {
  return self->GetIsolate();
}

v8::Value* v8__Exception__RangeError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::RangeError(message));
}

v8::Value* v8__Exception__ReferenceError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::ReferenceError(message));
}

v8::Value* v8__Exception__SyntaxError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::SyntaxError(message));
}

v8::Value* v8__Exception__TypeError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::TypeError(message));
}

v8::Value* v8__Exception__Error(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::Error(message));
}

v8::Message* v8__Exception__CreateMessage(v8::Isolate* isolate,
                                          v8::Local<v8::Value> exception) {
  return local_to_ptr(v8::Exception::CreateMessage(isolate, exception));
}

v8::StackTrace* v8__Exception__GetStackTrace(v8::Local<v8::Value> exception) {
  return local_to_ptr(v8::Exception::GetStackTrace(exception));
}

v8::Function* v8__Function__New(v8::Local<v8::Context> context,
                                v8::FunctionCallback callback) {
  return maybe_local_to_ptr(v8::Function::New(context, callback));
}

v8::Value* v8__Function__Call(v8::Function* self,
                              v8::Local<v8::Context> context,
                              v8::Local<v8::Value> recv, int argc,
                              v8::Local<v8::Value> argv[]) {
  return maybe_local_to_ptr(self->Call(context, recv, argc, argv));
}

v8::FunctionTemplate* v8__FunctionTemplate__New(
    v8::Isolate* isolate, v8::FunctionCallback callback = nullptr) {
  return local_to_ptr(v8::FunctionTemplate::New(isolate, callback));
}

v8::Function* v8__FunctionTemplate__GetFunction(
    v8::Local<v8::FunctionTemplate> self, v8::Local<v8::Context> context) {
  return maybe_local_to_ptr(self->GetFunction(context));
}
int v8__FunctionCallbackInfo__Length(
    v8::FunctionCallbackInfo<v8::Value>* self) {
  return self->Length();
}

v8::Isolate* v8__FunctionCallbackInfo__GetIsolate(
    v8::FunctionCallbackInfo<v8::Value>* self) {
  return self->GetIsolate();
}

void v8__FunctionCallbackInfo__GetReturnValue(
    v8::FunctionCallbackInfo<v8::Value>* self,
    v8::ReturnValue<v8::Value>* out) {
  *out = self->GetReturnValue();
}

void v8__ReturnValue__Set(v8::ReturnValue<v8::Value>* self,
                          v8::Local<v8::Value> value) {
  self->Set(value);
}

v8::Value* v8__ReturnValue__Get(v8::ReturnValue<v8::Value>* self) {
  return local_to_ptr(self->Get());
}

v8::Isolate* v8__ReturnValue__GetIsolate(v8::ReturnValue<v8::Value>* self) {
  return self->GetIsolate();
}

int v8__StackTrace__GetFrameCount(v8::StackTrace* self) {
  return self->GetFrameCount();
}

void v8__TryCatch__CONSTRUCT(uninit_t<v8::TryCatch>& buf,
                             v8::Isolate* isolate) {
  construct_in_place<v8::TryCatch>(buf, isolate);
}

void v8__TryCatch__DESTRUCT(v8::TryCatch& self) { self.~TryCatch(); }

bool v8__TryCatch__HasCaught(const v8::TryCatch& self) {
  return self.HasCaught();
}

bool v8__TryCatch__CanContinue(const v8::TryCatch& self) {
  return self.CanContinue();
}

bool v8__TryCatch__HasTerminated(const v8::TryCatch& self) {
  return self.HasTerminated();
}

v8::Value* v8__TryCatch__Exception(const v8::TryCatch& self) {
  return local_to_ptr(self.Exception());
}

v8::Value* v8__TryCatch__StackTrace(const v8::TryCatch& self,
                                    v8::Local<v8::Context> context) {
  return maybe_local_to_ptr(self.StackTrace(context));
}

v8::Message* v8__TryCatch__Message(const v8::TryCatch& self) {
  return local_to_ptr(self.Message());
}

void v8__TryCatch__Reset(v8::TryCatch& self) { self.Reset(); }

v8::Value* v8__TryCatch__ReThrow(v8::TryCatch& self) {
  return local_to_ptr(self.ReThrow());
}

bool v8__TryCatch__IsVerbose(const v8::TryCatch& self) {
  return self.IsVerbose();
}

void v8__TryCatch__SetVerbose(v8::TryCatch& self, bool value) {
  self.SetVerbose(value);
}

void v8__TryCatch__SetCaptureMessage(v8::TryCatch& self, bool value) {
  self.SetCaptureMessage(value);
}

v8::Script* v8__Script__Compile(v8::Context* context, v8::String* source,
                                v8::ScriptOrigin* origin) {
  return maybe_local_to_ptr(
      v8::Script::Compile(ptr_to_local(context), ptr_to_local(source), origin));
}

v8::Value* v8__Script__Run(v8::Script& script, v8::Context* context) {
  return maybe_local_to_ptr(script.Run(ptr_to_local(context)));
}

void v8__ScriptOrigin__CONSTRUCT(
    uninit_t<v8::ScriptOrigin>& buf, v8::Value* resource_name,
    v8::Integer* resource_line_offset, v8::Integer* resource_column_offset,
    v8::Boolean* resource_is_shared_cross_origin, v8::Integer* script_id,
    v8::Value* source_map_url, v8::Boolean* resource_is_opaque,
    v8::Boolean* is_wasm, v8::Boolean* is_module) {
  construct_in_place<v8::ScriptOrigin>(
      buf, ptr_to_local(resource_name), ptr_to_local(resource_line_offset),
      ptr_to_local(resource_column_offset),
      ptr_to_local(resource_is_shared_cross_origin), ptr_to_local(script_id),
      ptr_to_local(source_map_url), ptr_to_local(resource_is_opaque),
      ptr_to_local(is_wasm), ptr_to_local(is_module));
}

v8::Value* v8__JSON__Parse(v8::Local<v8::Context> context,
                           v8::Local<v8::String> json_string) {
  return maybe_local_to_ptr(v8::JSON::Parse(context, json_string));
}

v8::String* v8__JSON__Stringify(v8::Local<v8::Context> context,
                                v8::Local<v8::Value> json_object) {
  return maybe_local_to_ptr(v8::JSON::Stringify(context, json_object));
}

v8::Promise::Resolver* v8__Promise__Resolver__New(
    v8::Local<v8::Context> context) {
  return maybe_local_to_ptr(v8::Promise::Resolver::New(context));
}

v8::Promise* v8__Promise__Resolver__GetPromise(v8::Promise::Resolver* self) {
  return local_to_ptr(self->GetPromise());
}

MaybeBool v8__Promise__Resolver__Resolve(v8::Promise::Resolver* self,
                                         v8::Local<v8::Context> context,
                                         v8::Local<v8::Value> value) {
  return maybe_to_maybe_bool(self->Resolve(context, value));
}

MaybeBool v8__Promise__Resolver__Reject(v8::Promise::Resolver* self,
                                        v8::Local<v8::Context> context,
                                        v8::Local<v8::Value> value) {
  return maybe_to_maybe_bool(self->Reject(context, value));
}

v8::Promise::PromiseState v8__Promise__State(v8::Promise* self) {
  return self->State();
}

bool v8__Promise__HasHandler(v8::Promise* self) { return self->HasHandler(); }

v8::Value* v8__Promise__Result(v8::Promise* self) {
  return local_to_ptr(self->Result());
}

v8::Promise* v8__Promise__Catch(v8::Promise* self,
                                v8::Local<v8::Context> context,
                                v8::Local<v8::Function> handler) {
  return maybe_local_to_ptr(self->Catch(context, handler));
}

v8::Promise* v8__Promise__Then(v8::Promise* self,
                               v8::Local<v8::Context> context,
                               v8::Local<v8::Function> handler) {
  return maybe_local_to_ptr(self->Then(context, handler));
}

v8::Promise* v8__Promise__Then2(v8::Promise* self,
                                v8::Local<v8::Context> context,
                                v8::Local<v8::Function> on_fulfilled,
                                v8::Local<v8::Function> on_rejected) {
  return maybe_local_to_ptr(self->Then(context, on_fulfilled, on_rejected));
}

v8::PromiseRejectEvent v8__PromiseRejectMessage__GetEvent(
    const v8::PromiseRejectMessage& self) {
  return self.GetEvent();
}

v8::Promise* v8__PromiseRejectMessage__GetPromise(
    const v8::PromiseRejectMessage& self) {
  return local_to_ptr(self.GetPromise());
}

v8::Value* v8__PromiseRejectMessage__GetValue(
    const v8::PromiseRejectMessage& self) {
  return local_to_ptr(self.GetValue());
}

v8::Isolate* v8__PropertyCallbackInfo__GetIsolate(
    const v8::PropertyCallbackInfo<v8::Value>* self) {
  return self->GetIsolate();
}

v8::Object* v8__PropertyCallbackInfo__This(
    const v8::PropertyCallbackInfo<v8::Value>* self) {
  return local_to_ptr(self->This());
}

void v8__PropertyCallbackInfo__GetReturnValue(
    const v8::PropertyCallbackInfo<v8::Value>* self,
    v8::ReturnValue<v8::Value>* out) {
  *out = self->GetReturnValue();
}

v8::Platform* v8__platform__NewDefaultPlatform() {
  // TODO: support optional arguments.
  return v8::platform::NewDefaultPlatform().release();
}

void v8__Platform__DELETE(v8::Platform& self) { delete &self; }
void v8__Task__BASE__DELETE(v8::Task& self);
void v8__Task__BASE__Run(v8::Task& self);

struct v8__Task__BASE : public v8::Task {
  using Task::Task;
  void operator delete(void* ptr) noexcept {
    v8__Task__BASE__DELETE(*reinterpret_cast<v8::Task*>(ptr));
  }
  void Run() override { v8__Task__BASE__Run(*this); }
};

void v8__Task__BASE__CONSTRUCT(uninit_t<v8__Task__BASE>& buf) {
  construct_in_place<v8__Task__BASE>(buf);
}
void v8__Task__DELETE(v8::Task& self) { delete &self; }
void v8__Task__Run(v8::Task& self) { self.Run(); }

void v8_inspector__V8Inspector__Channel__BASE__sendResponse(
    v8_inspector::V8Inspector::Channel& self, int callId,
    v8_inspector::StringBuffer* message);
void v8_inspector__V8Inspector__Channel__BASE__sendNotification(
    v8_inspector::V8Inspector::Channel& self,
    v8_inspector::StringBuffer* message);
void v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(
    v8_inspector::V8Inspector::Channel& self);
}  // extern "C"

struct v8_inspector__V8Inspector__Channel__BASE
    : public v8_inspector::V8Inspector::Channel {
  using v8_inspector::V8Inspector::Channel::Channel;

  void sendResponse(
      int callId,
      std::unique_ptr<v8_inspector::StringBuffer> message) override {
    v8_inspector__V8Inspector__Channel__BASE__sendResponse(*this, callId,
                                                           message.release());
  }
  void sendNotification(
      std::unique_ptr<v8_inspector::StringBuffer> message) override {
    v8_inspector__V8Inspector__Channel__BASE__sendNotification(
        *this, message.release());
  }
  void flushProtocolNotifications() override {
    v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(*this);
  }
};

extern "C" {
void v8_inspector__V8Inspector__Channel__BASE__CONSTRUCT(
    uninit_t<v8_inspector__V8Inspector__Channel__BASE>& buf) {
  construct_in_place<v8_inspector__V8Inspector__Channel__BASE>(buf);
}

void v8_inspector__V8Inspector__Channel__sendResponse(
    v8_inspector::V8Inspector::Channel& self, int callId,
    v8_inspector::StringBuffer* message) {
  self.sendResponse(
      callId,
      static_cast<std::unique_ptr<v8_inspector::StringBuffer>>(message));
}
void v8_inspector__V8Inspector__Channel__sendNotification(
    v8_inspector::V8Inspector::Channel& self,
    v8_inspector::StringBuffer* message) {
  self.sendNotification(
      static_cast<std::unique_ptr<v8_inspector::StringBuffer>>(message));
}
void v8_inspector__V8Inspector__Channel__flushProtocolNotifications(
    v8_inspector::V8Inspector::Channel& self) {
  self.flushProtocolNotifications();
}

void v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
    v8_inspector::V8InspectorClient& self, int contextGroupId);
void v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(
    v8_inspector::V8InspectorClient& self);
void v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
    v8_inspector::V8InspectorClient& self, int contextGroupId);
}  // extern "C"

struct v8_inspector__V8InspectorClient__BASE
    : public v8_inspector::V8InspectorClient {
  using v8_inspector::V8InspectorClient::V8InspectorClient;

  void runMessageLoopOnPause(int contextGroupId) override {
    v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
        *this, contextGroupId);
  }
  void quitMessageLoopOnPause() override {
    v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(*this);
  }
  void runIfWaitingForDebugger(int contextGroupId) override {
    v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
        *this, contextGroupId);
  }
};

extern "C" {
void v8_inspector__V8InspectorClient__BASE__CONSTRUCT(
    uninit_t<v8_inspector__V8InspectorClient__BASE>& buf) {
  construct_in_place<v8_inspector__V8InspectorClient__BASE>(buf);
}

void v8_inspector__V8InspectorClient__runMessageLoopOnPause(
    v8_inspector::V8InspectorClient& self, int contextGroupId) {
  self.runMessageLoopOnPause(contextGroupId);
}
void v8_inspector__V8InspectorClient__quitMessageLoopOnPause(
    v8_inspector::V8InspectorClient& self) {
  self.quitMessageLoopOnPause();
}
void v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
    v8_inspector::V8InspectorClient& self, int contextGroupId) {
  self.runIfWaitingForDebugger(contextGroupId);
}

void v8_inspector__StringBuffer__DELETE(v8_inspector::StringBuffer& self) {
  delete &self;
}

const v8_inspector::StringView* v8_inspector__StringBuffer__string(
    v8_inspector::StringBuffer& self) {
  return &self.string();
}

v8_inspector::StringBuffer* v8_inspector__StringBuffer__create(
    const v8_inspector::StringView& source) {
  return v8_inspector::StringBuffer::create(source).release();
}
}  // extern "C"
