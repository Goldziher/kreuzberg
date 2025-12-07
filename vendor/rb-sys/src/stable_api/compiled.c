#include "ruby.h"
#include "ruby/ruby.h"
#include "ruby/intern.h"
#include "ruby/version.h"

// Check that we have Ruby API version macros
#if !defined(RUBY_API_VERSION_MAJOR)
#error "RUBY_API_VERSION_MAJOR is not defined, Ruby headers might not be configured correctly"
#endif

#if !defined(RUBY_API_VERSION_MINOR)
#error "RUBY_API_VERSION_MINOR is not defined, Ruby headers might not be configured correctly"
#endif

#if RUBY_API_VERSION_MAJOR > 3 || (RUBY_API_VERSION_MAJOR == 3 && RUBY_API_VERSION_MINOR >= 3)
#define RUBY_VERSION_AT_LEAST_3_3 1
#else
#define RUBY_VERSION_AT_LEAST_3_3 0
#endif

long impl_rstring_len(VALUE obj)
{
  return RSTRING_LEN(obj);
}

char *
impl_rstring_ptr(VALUE obj)
{
  return RSTRING_PTR(obj);
}

long impl_rarray_len(VALUE obj)
{
  return RARRAY_LEN(obj);
}

const VALUE *
impl_rarray_const_ptr(VALUE obj)
{
  return RARRAY_CONST_PTR(obj);
}

VALUE
impl_rbasic_class(VALUE obj)
{
  return RBASIC_CLASS(obj);
}

int impl_frozen_p(VALUE obj)
{
  return RB_OBJ_FROZEN(obj);
}

int impl_special_const_p(VALUE obj)
{
  return SPECIAL_CONST_P(obj);
}

int impl_bignum_positive_p(VALUE obj)
{
  return RBIGNUM_POSITIVE_P(obj);
}

int impl_bignum_negative_p(VALUE obj)
{
  return RBIGNUM_NEGATIVE_P(obj);
}

enum ruby_value_type
impl_builtin_type(VALUE obj)
{
  return RB_BUILTIN_TYPE(obj);
}

int impl_nil_p(VALUE obj)
{
  return NIL_P(obj);
}

int impl_fixnum_p(VALUE obj)
{
  return FIXNUM_P(obj);
}

int impl_static_sym_p(VALUE obj)
{
  return STATIC_SYM_P(obj);
}

int impl_flonum_p(VALUE obj)
{
  return FLONUM_P(obj);
}

int impl_immediate_p(VALUE obj)
{
  return IMMEDIATE_P(obj);
}

int impl_rb_test(VALUE obj)
{
  return RB_TEST(obj);
}

int impl_type_p(VALUE obj, enum ruby_value_type type)
{
  return RB_TYPE_P(obj, type);
}

int impl_dynamic_sym_p(VALUE obj)
{
  return RB_DYNAMIC_SYM_P(obj);
}

int impl_symbol_p(VALUE obj)
{
  return RB_SYMBOL_P(obj);
}

int impl_float_type_p(VALUE obj)
{
  return RB_FLOAT_TYPE_P(obj);
}

enum ruby_value_type
impl_rb_type(VALUE obj)
{
  return rb_type(obj);
}

int impl_integer_type_p(VALUE obj)
{
  return RB_INTEGER_TYPE_P(obj);
}

int impl_rstring_interned_p(VALUE obj)
{
  Check_Type(obj, T_STRING);

  return !(FL_TEST(obj, RSTRING_FSTR) == 0);
}

void impl_thread_sleep(struct timeval time)
{
  rb_thread_wait_for(time);
}

int impl_rtypeddata_p(VALUE obj)
{
  return RTYPEDDATA_P(obj);
}

int impl_rtypeddata_embedded_p(VALUE obj)
{
#if RUBY_VERSION_AT_LEAST_3_3
  return RTYPEDDATA_EMBEDDED_P(obj);
#else
  return 0;
#endif
}

const struct rb_data_type_struct *
impl_rtypeddata_type(VALUE obj)
{
  return RTYPEDDATA_TYPE(obj);
}

void *
impl_rtypeddata_get_data(VALUE obj)
{
#if RUBY_VERSION_AT_LEAST_3_3
  return RTYPEDDATA_GET_DATA(obj);
#else
  return RTYPEDDATA(obj)->data;
#endif
}
