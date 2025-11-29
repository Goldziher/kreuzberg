/*====================================================================*
 -  Copyright (C) 2001 Leptonica.  All rights reserved.
 -
 -  Redistribution and use in source and binary forms, with or without
 -  modification, are permitted provided that the following conditions
 -  are met:
 -  1. Redistributions of source code must retain the above copyright
 -     notice, this list of conditions and the following disclaimer.
 -  2. Redistributions in binary form must reproduce the above
 -     copyright notice, this list of conditions and the following
 -     disclaimer in the documentation and/or other materials
 -     provided with the distribution.
 -
 -  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 -  ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 -  LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 -  A PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL ANY
 -  CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 -  EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 -  PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 -  PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY
 -  OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 -  NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 -  SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *====================================================================*/

/*!
 *     Low-level fast binary morphology with auto-generated sels
 *
 *      Dispatcher:
 *             l_int32    fmorphopgen_low_3()
 *
 *      Static Low-level:
 *             void       fdilate_3_*()
 *             void       ferode_3_*()
 */

#include "allheaders.h"

static void  fdilate_3_0(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_0(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_1(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_1(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_2(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_2(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_3(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_3(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_4(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_4(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_5(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_5(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_6(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_6(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_7(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_7(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_8(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_8(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_9(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_9(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_10(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_10(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_11(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_11(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_12(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_12(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_13(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_13(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_14(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_14(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_15(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_15(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_16(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_16(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_17(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_17(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_18(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_18(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_19(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_19(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_20(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_20(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_21(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_21(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_22(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_22(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_23(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_23(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_24(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_24(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_25(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_25(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_26(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_26(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_27(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_27(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_28(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_28(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_29(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_29(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_30(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_30(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_31(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_31(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_32(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_32(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_33(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_33(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_34(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_34(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_35(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_35(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_36(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_36(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_37(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_37(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_38(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_38(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_39(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_39(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_40(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_40(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_41(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_41(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_42(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_42(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_43(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_43(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_44(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_44(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_45(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_45(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_46(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_46(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_47(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_47(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_48(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_48(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_49(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_49(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_50(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_50(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_51(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_51(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_52(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_52(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_53(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_53(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_54(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_54(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_55(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_55(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_56(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_56(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_57(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_57(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_58(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_58(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_59(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_59(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_60(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_60(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_61(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_61(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_62(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_62(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_63(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_63(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_64(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_64(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_65(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_65(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_66(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_66(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_67(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_67(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_68(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_68(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_69(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_69(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_70(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_70(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_71(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_71(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_72(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_72(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_73(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_73(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_74(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_74(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_75(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_75(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_76(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_76(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_77(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_77(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_78(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_78(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_79(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_79(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_80(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_80(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_81(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_81(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_82(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_82(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_83(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_83(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_84(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_84(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_85(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_85(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_86(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_86(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_87(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_87(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_88(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_88(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_89(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_89(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_90(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_90(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_91(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_91(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_92(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_92(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_93(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_93(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_94(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_94(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_95(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_95(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_96(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_96(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_97(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_97(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_98(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_98(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_99(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_99(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_100(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_100(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_101(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_101(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_102(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_102(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_103(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_103(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_104(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_104(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_105(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_105(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_106(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_106(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_107(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_107(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_108(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_108(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_109(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_109(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_110(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_110(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_111(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_111(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_112(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_112(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_113(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_113(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_114(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_114(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_115(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_115(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_116(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_116(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_117(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_117(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_118(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_118(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_119(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_119(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_120(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_120(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_121(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_121(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_122(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_122(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  fdilate_3_123(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);
static void  ferode_3_123(l_uint32 *, l_int32, l_int32, l_int32, l_uint32 *, l_int32);


/*---------------------------------------------------------------------*
 *                          Fast morph dispatcher                      *
 *---------------------------------------------------------------------*/
/*!
 *  fmorphopgen_low_3()
 *
 *       a dispatcher to appropriate low-level code
 */
l_int32
fmorphopgen_low_3(l_uint32  *datad,
                  l_int32    w,
                  l_int32    h,
                  l_int32    wpld,
                  l_uint32  *datas,
                  l_int32    wpls,
                  l_int32    index)
{

    switch (index)
    {
    case 0:
        fdilate_3_0(datad, w, h, wpld, datas, wpls);
        break;
    case 1:
        ferode_3_0(datad, w, h, wpld, datas, wpls);
        break;
    case 2:
        fdilate_3_1(datad, w, h, wpld, datas, wpls);
        break;
    case 3:
        ferode_3_1(datad, w, h, wpld, datas, wpls);
        break;
    case 4:
        fdilate_3_2(datad, w, h, wpld, datas, wpls);
        break;
    case 5:
        ferode_3_2(datad, w, h, wpld, datas, wpls);
        break;
    case 6:
        fdilate_3_3(datad, w, h, wpld, datas, wpls);
        break;
    case 7:
        ferode_3_3(datad, w, h, wpld, datas, wpls);
        break;
    case 8:
        fdilate_3_4(datad, w, h, wpld, datas, wpls);
        break;
    case 9:
        ferode_3_4(datad, w, h, wpld, datas, wpls);
        break;
    case 10:
        fdilate_3_5(datad, w, h, wpld, datas, wpls);
        break;
    case 11:
        ferode_3_5(datad, w, h, wpld, datas, wpls);
        break;
    case 12:
        fdilate_3_6(datad, w, h, wpld, datas, wpls);
        break;
    case 13:
        ferode_3_6(datad, w, h, wpld, datas, wpls);
        break;
    case 14:
        fdilate_3_7(datad, w, h, wpld, datas, wpls);
        break;
    case 15:
        ferode_3_7(datad, w, h, wpld, datas, wpls);
        break;
    case 16:
        fdilate_3_8(datad, w, h, wpld, datas, wpls);
        break;
    case 17:
        ferode_3_8(datad, w, h, wpld, datas, wpls);
        break;
    case 18:
        fdilate_3_9(datad, w, h, wpld, datas, wpls);
        break;
    case 19:
        ferode_3_9(datad, w, h, wpld, datas, wpls);
        break;
    case 20:
        fdilate_3_10(datad, w, h, wpld, datas, wpls);
        break;
    case 21:
        ferode_3_10(datad, w, h, wpld, datas, wpls);
        break;
    case 22:
        fdilate_3_11(datad, w, h, wpld, datas, wpls);
        break;
    case 23:
        ferode_3_11(datad, w, h, wpld, datas, wpls);
        break;
    case 24:
        fdilate_3_12(datad, w, h, wpld, datas, wpls);
        break;
    case 25:
        ferode_3_12(datad, w, h, wpld, datas, wpls);
        break;
    case 26:
        fdilate_3_13(datad, w, h, wpld, datas, wpls);
        break;
    case 27:
        ferode_3_13(datad, w, h, wpld, datas, wpls);
        break;
    case 28:
        fdilate_3_14(datad, w, h, wpld, datas, wpls);
        break;
    case 29:
        ferode_3_14(datad, w, h, wpld, datas, wpls);
        break;
    case 30:
        fdilate_3_15(datad, w, h, wpld, datas, wpls);
        break;
    case 31:
        ferode_3_15(datad, w, h, wpld, datas, wpls);
        break;
    case 32:
        fdilate_3_16(datad, w, h, wpld, datas, wpls);
        break;
    case 33:
        ferode_3_16(datad, w, h, wpld, datas, wpls);
        break;
    case 34:
        fdilate_3_17(datad, w, h, wpld, datas, wpls);
        break;
    case 35:
        ferode_3_17(datad, w, h, wpld, datas, wpls);
        break;
    case 36:
        fdilate_3_18(datad, w, h, wpld, datas, wpls);
        break;
    case 37:
        ferode_3_18(datad, w, h, wpld, datas, wpls);
        break;
    case 38:
        fdilate_3_19(datad, w, h, wpld, datas, wpls);
        break;
    case 39:
        ferode_3_19(datad, w, h, wpld, datas, wpls);
        break;
    case 40:
        fdilate_3_20(datad, w, h, wpld, datas, wpls);
        break;
    case 41:
        ferode_3_20(datad, w, h, wpld, datas, wpls);
        break;
    case 42:
        fdilate_3_21(datad, w, h, wpld, datas, wpls);
        break;
    case 43:
        ferode_3_21(datad, w, h, wpld, datas, wpls);
        break;
    case 44:
        fdilate_3_22(datad, w, h, wpld, datas, wpls);
        break;
    case 45:
        ferode_3_22(datad, w, h, wpld, datas, wpls);
        break;
    case 46:
        fdilate_3_23(datad, w, h, wpld, datas, wpls);
        break;
    case 47:
        ferode_3_23(datad, w, h, wpld, datas, wpls);
        break;
    case 48:
        fdilate_3_24(datad, w, h, wpld, datas, wpls);
        break;
    case 49:
        ferode_3_24(datad, w, h, wpld, datas, wpls);
        break;
    case 50:
        fdilate_3_25(datad, w, h, wpld, datas, wpls);
        break;
    case 51:
        ferode_3_25(datad, w, h, wpld, datas, wpls);
        break;
    case 52:
        fdilate_3_26(datad, w, h, wpld, datas, wpls);
        break;
    case 53:
        ferode_3_26(datad, w, h, wpld, datas, wpls);
        break;
    case 54:
        fdilate_3_27(datad, w, h, wpld, datas, wpls);
        break;
    case 55:
        ferode_3_27(datad, w, h, wpld, datas, wpls);
        break;
    case 56:
        fdilate_3_28(datad, w, h, wpld, datas, wpls);
        break;
    case 57:
        ferode_3_28(datad, w, h, wpld, datas, wpls);
        break;
    case 58:
        fdilate_3_29(datad, w, h, wpld, datas, wpls);
        break;
    case 59:
        ferode_3_29(datad, w, h, wpld, datas, wpls);
        break;
    case 60:
        fdilate_3_30(datad, w, h, wpld, datas, wpls);
        break;
    case 61:
        ferode_3_30(datad, w, h, wpld, datas, wpls);
        break;
    case 62:
        fdilate_3_31(datad, w, h, wpld, datas, wpls);
        break;
    case 63:
        ferode_3_31(datad, w, h, wpld, datas, wpls);
        break;
    case 64:
        fdilate_3_32(datad, w, h, wpld, datas, wpls);
        break;
    case 65:
        ferode_3_32(datad, w, h, wpld, datas, wpls);
        break;
    case 66:
        fdilate_3_33(datad, w, h, wpld, datas, wpls);
        break;
    case 67:
        ferode_3_33(datad, w, h, wpld, datas, wpls);
        break;
    case 68:
        fdilate_3_34(datad, w, h, wpld, datas, wpls);
        break;
    case 69:
        ferode_3_34(datad, w, h, wpld, datas, wpls);
        break;
    case 70:
        fdilate_3_35(datad, w, h, wpld, datas, wpls);
        break;
    case 71:
        ferode_3_35(datad, w, h, wpld, datas, wpls);
        break;
    case 72:
        fdilate_3_36(datad, w, h, wpld, datas, wpls);
        break;
    case 73:
        ferode_3_36(datad, w, h, wpld, datas, wpls);
        break;
    case 74:
        fdilate_3_37(datad, w, h, wpld, datas, wpls);
        break;
    case 75:
        ferode_3_37(datad, w, h, wpld, datas, wpls);
        break;
    case 76:
        fdilate_3_38(datad, w, h, wpld, datas, wpls);
        break;
    case 77:
        ferode_3_38(datad, w, h, wpld, datas, wpls);
        break;
    case 78:
        fdilate_3_39(datad, w, h, wpld, datas, wpls);
        break;
    case 79:
        ferode_3_39(datad, w, h, wpld, datas, wpls);
        break;
    case 80:
        fdilate_3_40(datad, w, h, wpld, datas, wpls);
        break;
    case 81:
        ferode_3_40(datad, w, h, wpld, datas, wpls);
        break;
    case 82:
        fdilate_3_41(datad, w, h, wpld, datas, wpls);
        break;
    case 83:
        ferode_3_41(datad, w, h, wpld, datas, wpls);
        break;
    case 84:
        fdilate_3_42(datad, w, h, wpld, datas, wpls);
        break;
    case 85:
        ferode_3_42(datad, w, h, wpld, datas, wpls);
        break;
    case 86:
        fdilate_3_43(datad, w, h, wpld, datas, wpls);
        break;
    case 87:
        ferode_3_43(datad, w, h, wpld, datas, wpls);
        break;
    case 88:
        fdilate_3_44(datad, w, h, wpld, datas, wpls);
        break;
    case 89:
        ferode_3_44(datad, w, h, wpld, datas, wpls);
        break;
    case 90:
        fdilate_3_45(datad, w, h, wpld, datas, wpls);
        break;
    case 91:
        ferode_3_45(datad, w, h, wpld, datas, wpls);
        break;
    case 92:
        fdilate_3_46(datad, w, h, wpld, datas, wpls);
        break;
    case 93:
        ferode_3_46(datad, w, h, wpld, datas, wpls);
        break;
    case 94:
        fdilate_3_47(datad, w, h, wpld, datas, wpls);
        break;
    case 95:
        ferode_3_47(datad, w, h, wpld, datas, wpls);
        break;
    case 96:
        fdilate_3_48(datad, w, h, wpld, datas, wpls);
        break;
    case 97:
        ferode_3_48(datad, w, h, wpld, datas, wpls);
        break;
    case 98:
        fdilate_3_49(datad, w, h, wpld, datas, wpls);
        break;
    case 99:
        ferode_3_49(datad, w, h, wpld, datas, wpls);
        break;
    case 100:
        fdilate_3_50(datad, w, h, wpld, datas, wpls);
        break;
    case 101:
        ferode_3_50(datad, w, h, wpld, datas, wpls);
        break;
    case 102:
        fdilate_3_51(datad, w, h, wpld, datas, wpls);
        break;
    case 103:
        ferode_3_51(datad, w, h, wpld, datas, wpls);
        break;
    case 104:
        fdilate_3_52(datad, w, h, wpld, datas, wpls);
        break;
    case 105:
        ferode_3_52(datad, w, h, wpld, datas, wpls);
        break;
    case 106:
        fdilate_3_53(datad, w, h, wpld, datas, wpls);
        break;
    case 107:
        ferode_3_53(datad, w, h, wpld, datas, wpls);
        break;
    case 108:
        fdilate_3_54(datad, w, h, wpld, datas, wpls);
        break;
    case 109:
        ferode_3_54(datad, w, h, wpld, datas, wpls);
        break;
    case 110:
        fdilate_3_55(datad, w, h, wpld, datas, wpls);
        break;
    case 111:
        ferode_3_55(datad, w, h, wpld, datas, wpls);
        break;
    case 112:
        fdilate_3_56(datad, w, h, wpld, datas, wpls);
        break;
    case 113:
        ferode_3_56(datad, w, h, wpld, datas, wpls);
        break;
    case 114:
        fdilate_3_57(datad, w, h, wpld, datas, wpls);
        break;
    case 115:
        ferode_3_57(datad, w, h, wpld, datas, wpls);
        break;
    case 116:
        fdilate_3_58(datad, w, h, wpld, datas, wpls);
        break;
    case 117:
        ferode_3_58(datad, w, h, wpld, datas, wpls);
        break;
    case 118:
        fdilate_3_59(datad, w, h, wpld, datas, wpls);
        break;
    case 119:
        ferode_3_59(datad, w, h, wpld, datas, wpls);
        break;
    case 120:
        fdilate_3_60(datad, w, h, wpld, datas, wpls);
        break;
    case 121:
        ferode_3_60(datad, w, h, wpld, datas, wpls);
        break;
    case 122:
        fdilate_3_61(datad, w, h, wpld, datas, wpls);
        break;
    case 123:
        ferode_3_61(datad, w, h, wpld, datas, wpls);
        break;
    case 124:
        fdilate_3_62(datad, w, h, wpld, datas, wpls);
        break;
    case 125:
        ferode_3_62(datad, w, h, wpld, datas, wpls);
        break;
    case 126:
        fdilate_3_63(datad, w, h, wpld, datas, wpls);
        break;
    case 127:
        ferode_3_63(datad, w, h, wpld, datas, wpls);
        break;
    case 128:
        fdilate_3_64(datad, w, h, wpld, datas, wpls);
        break;
    case 129:
        ferode_3_64(datad, w, h, wpld, datas, wpls);
        break;
    case 130:
        fdilate_3_65(datad, w, h, wpld, datas, wpls);
        break;
    case 131:
        ferode_3_65(datad, w, h, wpld, datas, wpls);
        break;
    case 132:
        fdilate_3_66(datad, w, h, wpld, datas, wpls);
        break;
    case 133:
        ferode_3_66(datad, w, h, wpld, datas, wpls);
        break;
    case 134:
        fdilate_3_67(datad, w, h, wpld, datas, wpls);
        break;
    case 135:
        ferode_3_67(datad, w, h, wpld, datas, wpls);
        break;
    case 136:
        fdilate_3_68(datad, w, h, wpld, datas, wpls);
        break;
    case 137:
        ferode_3_68(datad, w, h, wpld, datas, wpls);
        break;
    case 138:
        fdilate_3_69(datad, w, h, wpld, datas, wpls);
        break;
    case 139:
        ferode_3_69(datad, w, h, wpld, datas, wpls);
        break;
    case 140:
        fdilate_3_70(datad, w, h, wpld, datas, wpls);
        break;
    case 141:
        ferode_3_70(datad, w, h, wpld, datas, wpls);
        break;
    case 142:
        fdilate_3_71(datad, w, h, wpld, datas, wpls);
        break;
    case 143:
        ferode_3_71(datad, w, h, wpld, datas, wpls);
        break;
    case 144:
        fdilate_3_72(datad, w, h, wpld, datas, wpls);
        break;
    case 145:
        ferode_3_72(datad, w, h, wpld, datas, wpls);
        break;
    case 146:
        fdilate_3_73(datad, w, h, wpld, datas, wpls);
        break;
    case 147:
        ferode_3_73(datad, w, h, wpld, datas, wpls);
        break;
    case 148:
        fdilate_3_74(datad, w, h, wpld, datas, wpls);
        break;
    case 149:
        ferode_3_74(datad, w, h, wpld, datas, wpls);
        break;
    case 150:
        fdilate_3_75(datad, w, h, wpld, datas, wpls);
        break;
    case 151:
        ferode_3_75(datad, w, h, wpld, datas, wpls);
        break;
    case 152:
        fdilate_3_76(datad, w, h, wpld, datas, wpls);
        break;
    case 153:
        ferode_3_76(datad, w, h, wpld, datas, wpls);
        break;
    case 154:
        fdilate_3_77(datad, w, h, wpld, datas, wpls);
        break;
    case 155:
        ferode_3_77(datad, w, h, wpld, datas, wpls);
        break;
    case 156:
        fdilate_3_78(datad, w, h, wpld, datas, wpls);
        break;
    case 157:
        ferode_3_78(datad, w, h, wpld, datas, wpls);
        break;
    case 158:
        fdilate_3_79(datad, w, h, wpld, datas, wpls);
        break;
    case 159:
        ferode_3_79(datad, w, h, wpld, datas, wpls);
        break;
    case 160:
        fdilate_3_80(datad, w, h, wpld, datas, wpls);
        break;
    case 161:
        ferode_3_80(datad, w, h, wpld, datas, wpls);
        break;
    case 162:
        fdilate_3_81(datad, w, h, wpld, datas, wpls);
        break;
    case 163:
        ferode_3_81(datad, w, h, wpld, datas, wpls);
        break;
    case 164:
        fdilate_3_82(datad, w, h, wpld, datas, wpls);
        break;
    case 165:
        ferode_3_82(datad, w, h, wpld, datas, wpls);
        break;
    case 166:
        fdilate_3_83(datad, w, h, wpld, datas, wpls);
        break;
    case 167:
        ferode_3_83(datad, w, h, wpld, datas, wpls);
        break;
    case 168:
        fdilate_3_84(datad, w, h, wpld, datas, wpls);
        break;
    case 169:
        ferode_3_84(datad, w, h, wpld, datas, wpls);
        break;
    case 170:
        fdilate_3_85(datad, w, h, wpld, datas, wpls);
        break;
    case 171:
        ferode_3_85(datad, w, h, wpld, datas, wpls);
        break;
    case 172:
        fdilate_3_86(datad, w, h, wpld, datas, wpls);
        break;
    case 173:
        ferode_3_86(datad, w, h, wpld, datas, wpls);
        break;
    case 174:
        fdilate_3_87(datad, w, h, wpld, datas, wpls);
        break;
    case 175:
        ferode_3_87(datad, w, h, wpld, datas, wpls);
        break;
    case 176:
        fdilate_3_88(datad, w, h, wpld, datas, wpls);
        break;
    case 177:
        ferode_3_88(datad, w, h, wpld, datas, wpls);
        break;
    case 178:
        fdilate_3_89(datad, w, h, wpld, datas, wpls);
        break;
    case 179:
        ferode_3_89(datad, w, h, wpld, datas, wpls);
        break;
    case 180:
        fdilate_3_90(datad, w, h, wpld, datas, wpls);
        break;
    case 181:
        ferode_3_90(datad, w, h, wpld, datas, wpls);
        break;
    case 182:
        fdilate_3_91(datad, w, h, wpld, datas, wpls);
        break;
    case 183:
        ferode_3_91(datad, w, h, wpld, datas, wpls);
        break;
    case 184:
        fdilate_3_92(datad, w, h, wpld, datas, wpls);
        break;
    case 185:
        ferode_3_92(datad, w, h, wpld, datas, wpls);
        break;
    case 186:
        fdilate_3_93(datad, w, h, wpld, datas, wpls);
        break;
    case 187:
        ferode_3_93(datad, w, h, wpld, datas, wpls);
        break;
    case 188:
        fdilate_3_94(datad, w, h, wpld, datas, wpls);
        break;
    case 189:
        ferode_3_94(datad, w, h, wpld, datas, wpls);
        break;
    case 190:
        fdilate_3_95(datad, w, h, wpld, datas, wpls);
        break;
    case 191:
        ferode_3_95(datad, w, h, wpld, datas, wpls);
        break;
    case 192:
        fdilate_3_96(datad, w, h, wpld, datas, wpls);
        break;
    case 193:
        ferode_3_96(datad, w, h, wpld, datas, wpls);
        break;
    case 194:
        fdilate_3_97(datad, w, h, wpld, datas, wpls);
        break;
    case 195:
        ferode_3_97(datad, w, h, wpld, datas, wpls);
        break;
    case 196:
        fdilate_3_98(datad, w, h, wpld, datas, wpls);
        break;
    case 197:
        ferode_3_98(datad, w, h, wpld, datas, wpls);
        break;
    case 198:
        fdilate_3_99(datad, w, h, wpld, datas, wpls);
        break;
    case 199:
        ferode_3_99(datad, w, h, wpld, datas, wpls);
        break;
    case 200:
        fdilate_3_100(datad, w, h, wpld, datas, wpls);
        break;
    case 201:
        ferode_3_100(datad, w, h, wpld, datas, wpls);
        break;
    case 202:
        fdilate_3_101(datad, w, h, wpld, datas, wpls);
        break;
    case 203:
        ferode_3_101(datad, w, h, wpld, datas, wpls);
        break;
    case 204:
        fdilate_3_102(datad, w, h, wpld, datas, wpls);
        break;
    case 205:
        ferode_3_102(datad, w, h, wpld, datas, wpls);
        break;
    case 206:
        fdilate_3_103(datad, w, h, wpld, datas, wpls);
        break;
    case 207:
        ferode_3_103(datad, w, h, wpld, datas, wpls);
        break;
    case 208:
        fdilate_3_104(datad, w, h, wpld, datas, wpls);
        break;
    case 209:
        ferode_3_104(datad, w, h, wpld, datas, wpls);
        break;
    case 210:
        fdilate_3_105(datad, w, h, wpld, datas, wpls);
        break;
    case 211:
        ferode_3_105(datad, w, h, wpld, datas, wpls);
        break;
    case 212:
        fdilate_3_106(datad, w, h, wpld, datas, wpls);
        break;
    case 213:
        ferode_3_106(datad, w, h, wpld, datas, wpls);
        break;
    case 214:
        fdilate_3_107(datad, w, h, wpld, datas, wpls);
        break;
    case 215:
        ferode_3_107(datad, w, h, wpld, datas, wpls);
        break;
    case 216:
        fdilate_3_108(datad, w, h, wpld, datas, wpls);
        break;
    case 217:
        ferode_3_108(datad, w, h, wpld, datas, wpls);
        break;
    case 218:
        fdilate_3_109(datad, w, h, wpld, datas, wpls);
        break;
    case 219:
        ferode_3_109(datad, w, h, wpld, datas, wpls);
        break;
    case 220:
        fdilate_3_110(datad, w, h, wpld, datas, wpls);
        break;
    case 221:
        ferode_3_110(datad, w, h, wpld, datas, wpls);
        break;
    case 222:
        fdilate_3_111(datad, w, h, wpld, datas, wpls);
        break;
    case 223:
        ferode_3_111(datad, w, h, wpld, datas, wpls);
        break;
    case 224:
        fdilate_3_112(datad, w, h, wpld, datas, wpls);
        break;
    case 225:
        ferode_3_112(datad, w, h, wpld, datas, wpls);
        break;
    case 226:
        fdilate_3_113(datad, w, h, wpld, datas, wpls);
        break;
    case 227:
        ferode_3_113(datad, w, h, wpld, datas, wpls);
        break;
    case 228:
        fdilate_3_114(datad, w, h, wpld, datas, wpls);
        break;
    case 229:
        ferode_3_114(datad, w, h, wpld, datas, wpls);
        break;
    case 230:
        fdilate_3_115(datad, w, h, wpld, datas, wpls);
        break;
    case 231:
        ferode_3_115(datad, w, h, wpld, datas, wpls);
        break;
    case 232:
        fdilate_3_116(datad, w, h, wpld, datas, wpls);
        break;
    case 233:
        ferode_3_116(datad, w, h, wpld, datas, wpls);
        break;
    case 234:
        fdilate_3_117(datad, w, h, wpld, datas, wpls);
        break;
    case 235:
        ferode_3_117(datad, w, h, wpld, datas, wpls);
        break;
    case 236:
        fdilate_3_118(datad, w, h, wpld, datas, wpls);
        break;
    case 237:
        ferode_3_118(datad, w, h, wpld, datas, wpls);
        break;
    case 238:
        fdilate_3_119(datad, w, h, wpld, datas, wpls);
        break;
    case 239:
        ferode_3_119(datad, w, h, wpld, datas, wpls);
        break;
    case 240:
        fdilate_3_120(datad, w, h, wpld, datas, wpls);
        break;
    case 241:
        ferode_3_120(datad, w, h, wpld, datas, wpls);
        break;
    case 242:
        fdilate_3_121(datad, w, h, wpld, datas, wpls);
        break;
    case 243:
        ferode_3_121(datad, w, h, wpld, datas, wpls);
        break;
    case 244:
        fdilate_3_122(datad, w, h, wpld, datas, wpls);
        break;
    case 245:
        ferode_3_122(datad, w, h, wpld, datas, wpls);
        break;
    case 246:
        fdilate_3_123(datad, w, h, wpld, datas, wpls);
        break;
    case 247:
        ferode_3_123(datad, w, h, wpld, datas, wpls);
        break;
    }

    return 0;
}


/*--------------------------------------------------------------------------*
 *                 Low-level auto-generated static routines                 *
 *--------------------------------------------------------------------------*/
/*
 *  N.B.  In all the low-level routines, the part of the image
 *        that is accessed has been clipped by 32 pixels on
 *        all four sides.  This is done in the higher level
 *        code by redefining w and h smaller and by moving the
 *        start-of-image pointers up to the beginning of this
 *        interior rectangle.
 */
static void
fdilate_3_0(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr);
        }
    }
}

static void
ferode_3_0(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr);
        }
    }
}

static void
fdilate_3_1(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31));
        }
    }
}

static void
ferode_3_1(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31));
        }
    }
}

static void
fdilate_3_2(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31));
        }
    }
}

static void
ferode_3_2(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31));
        }
    }
}

static void
fdilate_3_3(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30));
        }
    }
}

static void
ferode_3_3(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30));
        }
    }
}

static void
fdilate_3_4(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30));
        }
    }
}

static void
ferode_3_4(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30));
        }
    }
}

static void
fdilate_3_5(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29));
        }
    }
}

static void
ferode_3_5(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29));
        }
    }
}

static void
fdilate_3_6(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29));
        }
    }
}

static void
ferode_3_6(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29));
        }
    }
}

static void
fdilate_3_7(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28));
        }
    }
}

static void
ferode_3_7(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28));
        }
    }
}

static void
fdilate_3_8(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28));
        }
    }
}

static void
ferode_3_8(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28));
        }
    }
}

static void
fdilate_3_9(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27));
        }
    }
}

static void
ferode_3_9(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27));
        }
    }
}

static void
fdilate_3_10(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27));
        }
    }
}

static void
ferode_3_10(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27));
        }
    }
}

static void
fdilate_3_11(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26));
        }
    }
}

static void
ferode_3_11(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26));
        }
    }
}

static void
fdilate_3_12(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26));
        }
    }
}

static void
ferode_3_12(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26));
        }
    }
}

static void
fdilate_3_13(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25));
        }
    }
}

static void
ferode_3_13(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25));
        }
    }
}

static void
fdilate_3_14(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25));
        }
    }
}

static void
ferode_3_14(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25));
        }
    }
}

static void
fdilate_3_15(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24));
        }
    }
}

static void
ferode_3_15(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24));
        }
    }
}

static void
fdilate_3_16(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24));
        }
    }
}

static void
ferode_3_16(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24));
        }
    }
}

static void
fdilate_3_17(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23));
        }
    }
}

static void
ferode_3_17(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23));
        }
    }
}

static void
fdilate_3_18(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23));
        }
    }
}

static void
ferode_3_18(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23));
        }
    }
}

static void
fdilate_3_19(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22));
        }
    }
}

static void
ferode_3_19(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22));
        }
    }
}

static void
fdilate_3_20(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22));
        }
    }
}

static void
ferode_3_20(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22));
        }
    }
}

static void
fdilate_3_21(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21));
        }
    }
}

static void
ferode_3_21(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21));
        }
    }
}

static void
fdilate_3_22(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21));
        }
    }
}

static void
ferode_3_22(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21));
        }
    }
}

static void
fdilate_3_23(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20));
        }
    }
}

static void
ferode_3_23(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20));
        }
    }
}

static void
fdilate_3_24(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20));
        }
    }
}

static void
ferode_3_24(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20));
        }
    }
}

static void
fdilate_3_25(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19));
        }
    }
}

static void
ferode_3_25(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19));
        }
    }
}

static void
fdilate_3_26(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19));
        }
    }
}

static void
ferode_3_26(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19));
        }
    }
}

static void
fdilate_3_27(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18));
        }
    }
}

static void
ferode_3_27(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18));
        }
    }
}

static void
fdilate_3_28(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18));
        }
    }
}

static void
ferode_3_28(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18));
        }
    }
}

static void
fdilate_3_29(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17));
        }
    }
}

static void
ferode_3_29(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17));
        }
    }
}

static void
fdilate_3_30(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17));
        }
    }
}

static void
ferode_3_30(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17));
        }
    }
}

static void
fdilate_3_31(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16));
        }
    }
}

static void
ferode_3_31(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16));
        }
    }
}

static void
fdilate_3_32(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16));
        }
    }
}

static void
ferode_3_32(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16));
        }
    }
}

static void
fdilate_3_33(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15));
        }
    }
}

static void
ferode_3_33(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15));
        }
    }
}

static void
fdilate_3_34(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15));
        }
    }
}

static void
ferode_3_34(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15));
        }
    }
}

static void
fdilate_3_35(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14));
        }
    }
}

static void
ferode_3_35(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14));
        }
    }
}

static void
fdilate_3_36(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14));
        }
    }
}

static void
ferode_3_36(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14));
        }
    }
}

static void
fdilate_3_37(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13));
        }
    }
}

static void
ferode_3_37(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13));
        }
    }
}

static void
fdilate_3_38(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13));
        }
    }
}

static void
ferode_3_38(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13));
        }
    }
}

static void
fdilate_3_39(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12));
        }
    }
}

static void
ferode_3_39(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12));
        }
    }
}

static void
fdilate_3_40(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12));
        }
    }
}

static void
ferode_3_40(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12));
        }
    }
}

static void
fdilate_3_41(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11));
        }
    }
}

static void
ferode_3_41(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11));
        }
    }
}

static void
fdilate_3_42(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11));
        }
    }
}

static void
ferode_3_42(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11));
        }
    }
}

static void
fdilate_3_43(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10));
        }
    }
}

static void
ferode_3_43(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10));
        }
    }
}

static void
fdilate_3_44(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10));
        }
    }
}

static void
ferode_3_44(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10));
        }
    }
}

static void
fdilate_3_45(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9));
        }
    }
}

static void
ferode_3_45(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9));
        }
    }
}

static void
fdilate_3_46(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9));
        }
    }
}

static void
ferode_3_46(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9));
        }
    }
}

static void
fdilate_3_47(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8));
        }
    }
}

static void
ferode_3_47(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8));
        }
    }
}

static void
fdilate_3_48(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8));
        }
    }
}

static void
ferode_3_48(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8));
        }
    }
}

static void
fdilate_3_49(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7));
        }
    }
}

static void
ferode_3_49(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7));
        }
    }
}

static void
fdilate_3_50(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7));
        }
    }
}

static void
ferode_3_50(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7));
        }
    }
}

static void
fdilate_3_51(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6));
        }
    }
}

static void
ferode_3_51(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6));
        }
    }
}

static void
fdilate_3_52(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6));
        }
    }
}

static void
ferode_3_52(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6));
        }
    }
}

static void
fdilate_3_53(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) |
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5));
        }
    }
}

static void
ferode_3_53(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) &
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5));
        }
    }
}

static void
fdilate_3_54(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 28) | (*(sptr + 1) >> 4)) |
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) |
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5));
        }
    }
}

static void
ferode_3_54(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 28) | (*(sptr - 1) << 4)) &
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) &
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5));
        }
    }
}

static void
fdilate_3_55(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 28) | (*(sptr + 1) >> 4)) |
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) |
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) |
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4));
        }
    }
}

static void
ferode_3_55(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 28) | (*(sptr - 1) << 4)) &
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) &
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) &
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4));
        }
    }
}

static void
fdilate_3_56(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 29) | (*(sptr + 1) >> 3)) |
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) |
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) |
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) |
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4));
        }
    }
}

static void
ferode_3_56(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 29) | (*(sptr - 1) << 3)) &
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) &
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) &
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) &
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4));
        }
    }
}

static void
fdilate_3_57(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 29) | (*(sptr + 1) >> 3)) |
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) |
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) |
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) |
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) |
                    ((*(sptr) >> 29) | (*(sptr - 1) << 3));
        }
    }
}

static void
ferode_3_57(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 29) | (*(sptr - 1) << 3)) &
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) &
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) &
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) &
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) &
                    ((*(sptr) << 29) | (*(sptr + 1) >> 3));
        }
    }
}

static void
fdilate_3_58(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 30) | (*(sptr + 1) >> 2)) |
                    ((*(sptr) << 29) | (*(sptr + 1) >> 3)) |
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) |
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) |
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) |
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) |
                    ((*(sptr) >> 29) | (*(sptr - 1) << 3));
        }
    }
}

static void
ferode_3_58(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 30) | (*(sptr - 1) << 2)) &
                    ((*(sptr) >> 29) | (*(sptr - 1) << 3)) &
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) &
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) &
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) &
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) &
                    ((*(sptr) << 29) | (*(sptr + 1) >> 3));
        }
    }
}

static void
fdilate_3_59(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 30) | (*(sptr + 1) >> 2)) |
                    ((*(sptr) << 29) | (*(sptr + 1) >> 3)) |
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) |
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) |
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) |
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) |
                    ((*(sptr) >> 29) | (*(sptr - 1) << 3)) |
                    ((*(sptr) >> 30) | (*(sptr - 1) << 2));
        }
    }
}

static void
ferode_3_59(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 30) | (*(sptr - 1) << 2)) &
                    ((*(sptr) >> 29) | (*(sptr - 1) << 3)) &
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) &
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) &
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) &
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) &
                    ((*(sptr) << 29) | (*(sptr + 1) >> 3)) &
                    ((*(sptr) << 30) | (*(sptr + 1) >> 2));
        }
    }
}

static void
fdilate_3_60(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 31) | (*(sptr + 1) >> 1)) |
                    ((*(sptr) << 30) | (*(sptr + 1) >> 2)) |
                    ((*(sptr) << 29) | (*(sptr + 1) >> 3)) |
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) |
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) |
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) |
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) |
                    ((*(sptr) >> 29) | (*(sptr - 1) << 3)) |
                    ((*(sptr) >> 30) | (*(sptr - 1) << 2));
        }
    }
}

static void
ferode_3_60(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 31) | (*(sptr - 1) << 1)) &
                    ((*(sptr) >> 30) | (*(sptr - 1) << 2)) &
                    ((*(sptr) >> 29) | (*(sptr - 1) << 3)) &
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) &
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) &
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) &
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) &
                    ((*(sptr) << 29) | (*(sptr + 1) >> 3)) &
                    ((*(sptr) << 30) | (*(sptr + 1) >> 2));
        }
    }
}

static void
fdilate_3_61(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) << 31) | (*(sptr + 1) >> 1)) |
                    ((*(sptr) << 30) | (*(sptr + 1) >> 2)) |
                    ((*(sptr) << 29) | (*(sptr + 1) >> 3)) |
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) |
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) |
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) |
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) |
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) |
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) |
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) |
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) |
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) |
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) |
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) |
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) |
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) |
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) |
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) |
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) |
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) |
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) |
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) |
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) |
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) |
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) |
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) |
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) |
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) |
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) |
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) |
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) |
                    (*sptr) |
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) |
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) |
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) |
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) |
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) |
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) |
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) |
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) |
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) |
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) |
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) |
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) |
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) |
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) |
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) |
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) |
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) |
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) |
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) |
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) |
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) |
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) |
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) |
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) |
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) |
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) |
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) |
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) |
                    ((*(sptr) >> 29) | (*(sptr - 1) << 3)) |
                    ((*(sptr) >> 30) | (*(sptr - 1) << 2)) |
                    ((*(sptr) >> 31) | (*(sptr - 1) << 1));
        }
    }
}

static void
ferode_3_61(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = ((*(sptr) >> 31) | (*(sptr - 1) << 1)) &
                    ((*(sptr) >> 30) | (*(sptr - 1) << 2)) &
                    ((*(sptr) >> 29) | (*(sptr - 1) << 3)) &
                    ((*(sptr) >> 28) | (*(sptr - 1) << 4)) &
                    ((*(sptr) >> 27) | (*(sptr - 1) << 5)) &
                    ((*(sptr) >> 26) | (*(sptr - 1) << 6)) &
                    ((*(sptr) >> 25) | (*(sptr - 1) << 7)) &
                    ((*(sptr) >> 24) | (*(sptr - 1) << 8)) &
                    ((*(sptr) >> 23) | (*(sptr - 1) << 9)) &
                    ((*(sptr) >> 22) | (*(sptr - 1) << 10)) &
                    ((*(sptr) >> 21) | (*(sptr - 1) << 11)) &
                    ((*(sptr) >> 20) | (*(sptr - 1) << 12)) &
                    ((*(sptr) >> 19) | (*(sptr - 1) << 13)) &
                    ((*(sptr) >> 18) | (*(sptr - 1) << 14)) &
                    ((*(sptr) >> 17) | (*(sptr - 1) << 15)) &
                    ((*(sptr) >> 16) | (*(sptr - 1) << 16)) &
                    ((*(sptr) >> 15) | (*(sptr - 1) << 17)) &
                    ((*(sptr) >> 14) | (*(sptr - 1) << 18)) &
                    ((*(sptr) >> 13) | (*(sptr - 1) << 19)) &
                    ((*(sptr) >> 12) | (*(sptr - 1) << 20)) &
                    ((*(sptr) >> 11) | (*(sptr - 1) << 21)) &
                    ((*(sptr) >> 10) | (*(sptr - 1) << 22)) &
                    ((*(sptr) >> 9) | (*(sptr - 1) << 23)) &
                    ((*(sptr) >> 8) | (*(sptr - 1) << 24)) &
                    ((*(sptr) >> 7) | (*(sptr - 1) << 25)) &
                    ((*(sptr) >> 6) | (*(sptr - 1) << 26)) &
                    ((*(sptr) >> 5) | (*(sptr - 1) << 27)) &
                    ((*(sptr) >> 4) | (*(sptr - 1) << 28)) &
                    ((*(sptr) >> 3) | (*(sptr - 1) << 29)) &
                    ((*(sptr) >> 2) | (*(sptr - 1) << 30)) &
                    ((*(sptr) >> 1) | (*(sptr - 1) << 31)) &
                    (*sptr) &
                    ((*(sptr) << 1) | (*(sptr + 1) >> 31)) &
                    ((*(sptr) << 2) | (*(sptr + 1) >> 30)) &
                    ((*(sptr) << 3) | (*(sptr + 1) >> 29)) &
                    ((*(sptr) << 4) | (*(sptr + 1) >> 28)) &
                    ((*(sptr) << 5) | (*(sptr + 1) >> 27)) &
                    ((*(sptr) << 6) | (*(sptr + 1) >> 26)) &
                    ((*(sptr) << 7) | (*(sptr + 1) >> 25)) &
                    ((*(sptr) << 8) | (*(sptr + 1) >> 24)) &
                    ((*(sptr) << 9) | (*(sptr + 1) >> 23)) &
                    ((*(sptr) << 10) | (*(sptr + 1) >> 22)) &
                    ((*(sptr) << 11) | (*(sptr + 1) >> 21)) &
                    ((*(sptr) << 12) | (*(sptr + 1) >> 20)) &
                    ((*(sptr) << 13) | (*(sptr + 1) >> 19)) &
                    ((*(sptr) << 14) | (*(sptr + 1) >> 18)) &
                    ((*(sptr) << 15) | (*(sptr + 1) >> 17)) &
                    ((*(sptr) << 16) | (*(sptr + 1) >> 16)) &
                    ((*(sptr) << 17) | (*(sptr + 1) >> 15)) &
                    ((*(sptr) << 18) | (*(sptr + 1) >> 14)) &
                    ((*(sptr) << 19) | (*(sptr + 1) >> 13)) &
                    ((*(sptr) << 20) | (*(sptr + 1) >> 12)) &
                    ((*(sptr) << 21) | (*(sptr + 1) >> 11)) &
                    ((*(sptr) << 22) | (*(sptr + 1) >> 10)) &
                    ((*(sptr) << 23) | (*(sptr + 1) >> 9)) &
                    ((*(sptr) << 24) | (*(sptr + 1) >> 8)) &
                    ((*(sptr) << 25) | (*(sptr + 1) >> 7)) &
                    ((*(sptr) << 26) | (*(sptr + 1) >> 6)) &
                    ((*(sptr) << 27) | (*(sptr + 1) >> 5)) &
                    ((*(sptr) << 28) | (*(sptr + 1) >> 4)) &
                    ((*(sptr) << 29) | (*(sptr + 1) >> 3)) &
                    ((*(sptr) << 30) | (*(sptr + 1) >> 2)) &
                    ((*(sptr) << 31) | (*(sptr + 1) >> 1));
        }
    }
}

static void
fdilate_3_62(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls)) |
                    (*sptr);
        }
    }
}

static void
ferode_3_62(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls)) &
                    (*sptr);
        }
    }
}

static void
fdilate_3_63(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls));
        }
    }
}

static void
ferode_3_63(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;

    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls));
        }
    }
}

static void
fdilate_3_64(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2;

    wpls2 = 2 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls));
        }
    }
}

static void
ferode_3_64(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2;

    wpls2 = 2 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls));
        }
    }
}

static void
fdilate_3_65(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2;

    wpls2 = 2 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2));
        }
    }
}

static void
ferode_3_65(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2;

    wpls2 = 2 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2));
        }
    }
}

static void
fdilate_3_66(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2));
        }
    }
}

static void
ferode_3_66(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2));
        }
    }
}

static void
fdilate_3_67(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3));
        }
    }
}

static void
ferode_3_67(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3));
        }
    }
}

static void
fdilate_3_68(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3));
        }
    }
}

static void
ferode_3_68(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3));
        }
    }
}

static void
fdilate_3_69(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4));
        }
    }
}

static void
ferode_3_69(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4));
        }
    }
}

static void
fdilate_3_70(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4));
        }
    }
}

static void
ferode_3_70(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4));
        }
    }
}

static void
fdilate_3_71(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5));
        }
    }
}

static void
ferode_3_71(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5));
        }
    }
}

static void
fdilate_3_72(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5));
        }
    }
}

static void
ferode_3_72(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5));
        }
    }
}

static void
fdilate_3_73(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6));
        }
    }
}

static void
ferode_3_73(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6));
        }
    }
}

static void
fdilate_3_74(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6));
        }
    }
}

static void
ferode_3_74(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6));
        }
    }
}

static void
fdilate_3_75(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7));
        }
    }
}

static void
ferode_3_75(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7));
        }
    }
}

static void
fdilate_3_76(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7));
        }
    }
}

static void
ferode_3_76(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7));
        }
    }
}

static void
fdilate_3_77(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8));
        }
    }
}

static void
ferode_3_77(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8));
        }
    }
}

static void
fdilate_3_78(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8));
        }
    }
}

static void
ferode_3_78(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8));
        }
    }
}

static void
fdilate_3_79(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9));
        }
    }
}

static void
ferode_3_79(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9));
        }
    }
}

static void
fdilate_3_80(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9));
        }
    }
}

static void
ferode_3_80(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9));
        }
    }
}

static void
fdilate_3_81(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10));
        }
    }
}

static void
ferode_3_81(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10));
        }
    }
}

static void
fdilate_3_82(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10));
        }
    }
}

static void
ferode_3_82(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10));
        }
    }
}

static void
fdilate_3_83(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11));
        }
    }
}

static void
ferode_3_83(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11));
        }
    }
}

static void
fdilate_3_84(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11));
        }
    }
}

static void
ferode_3_84(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11));
        }
    }
}

static void
fdilate_3_85(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12));
        }
    }
}

static void
ferode_3_85(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12));
        }
    }
}

static void
fdilate_3_86(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12));
        }
    }
}

static void
ferode_3_86(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12));
        }
    }
}

static void
fdilate_3_87(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13));
        }
    }
}

static void
ferode_3_87(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13));
        }
    }
}

static void
fdilate_3_88(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13));
        }
    }
}

static void
ferode_3_88(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13));
        }
    }
}

static void
fdilate_3_89(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14));
        }
    }
}

static void
ferode_3_89(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14));
        }
    }
}

static void
fdilate_3_90(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14));
        }
    }
}

static void
ferode_3_90(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14));
        }
    }
}

static void
fdilate_3_91(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15));
        }
    }
}

static void
ferode_3_91(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15));
        }
    }
}

static void
fdilate_3_92(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15));
        }
    }
}

static void
ferode_3_92(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15));
        }
    }
}

static void
fdilate_3_93(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16));
        }
    }
}

static void
ferode_3_93(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16));
        }
    }
}

static void
fdilate_3_94(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16));
        }
    }
}

static void
ferode_3_94(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16));
        }
    }
}

static void
fdilate_3_95(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17));
        }
    }
}

static void
ferode_3_95(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17));
        }
    }
}

static void
fdilate_3_96(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17));
        }
    }
}

static void
ferode_3_96(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17));
        }
    }
}

static void
fdilate_3_97(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18));
        }
    }
}

static void
ferode_3_97(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18));
        }
    }
}

static void
fdilate_3_98(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18));
        }
    }
}

static void
ferode_3_98(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18));
        }
    }
}

static void
fdilate_3_99(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19));
        }
    }
}

static void
ferode_3_99(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19));
        }
    }
}

static void
fdilate_3_100(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19));
        }
    }
}

static void
ferode_3_100(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19));
        }
    }
}

static void
fdilate_3_101(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20));
        }
    }
}

static void
ferode_3_101(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20));
        }
    }
}

static void
fdilate_3_102(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20));
        }
    }
}

static void
ferode_3_102(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20));
        }
    }
}

static void
fdilate_3_103(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21));
        }
    }
}

static void
ferode_3_103(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21));
        }
    }
}

static void
fdilate_3_104(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21));
        }
    }
}

static void
ferode_3_104(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21));
        }
    }
}

static void
fdilate_3_105(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22));
        }
    }
}

static void
ferode_3_105(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22));
        }
    }
}

static void
fdilate_3_106(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22));
        }
    }
}

static void
ferode_3_106(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22));
        }
    }
}

static void
fdilate_3_107(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23));
        }
    }
}

static void
ferode_3_107(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23));
        }
    }
}

static void
fdilate_3_108(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23));
        }
    }
}

static void
ferode_3_108(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23));
        }
    }
}

static void
fdilate_3_109(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24));
        }
    }
}

static void
ferode_3_109(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24));
        }
    }
}

static void
fdilate_3_110(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24));
        }
    }
}

static void
ferode_3_110(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24));
        }
    }
}

static void
fdilate_3_111(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25));
        }
    }
}

static void
ferode_3_111(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25));
        }
    }
}

static void
fdilate_3_112(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25));
        }
    }
}

static void
ferode_3_112(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25));
        }
    }
}

static void
fdilate_3_113(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26));
        }
    }
}

static void
ferode_3_113(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26));
        }
    }
}

static void
fdilate_3_114(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26));
        }
    }
}

static void
ferode_3_114(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26));
        }
    }
}

static void
fdilate_3_115(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26)) |
                    (*(sptr - wpls27));
        }
    }
}

static void
ferode_3_115(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26)) &
                    (*(sptr + wpls27));
        }
    }
}

static void
fdilate_3_116(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls28)) |
                    (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26)) |
                    (*(sptr - wpls27));
        }
    }
}

static void
ferode_3_116(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls28)) &
                    (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26)) &
                    (*(sptr + wpls27));
        }
    }
}

static void
fdilate_3_117(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls28)) |
                    (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26)) |
                    (*(sptr - wpls27)) |
                    (*(sptr - wpls28));
        }
    }
}

static void
ferode_3_117(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls28)) &
                    (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26)) &
                    (*(sptr + wpls27)) &
                    (*(sptr + wpls28));
        }
    }
}

static void
fdilate_3_118(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls29)) |
                    (*(sptr + wpls28)) |
                    (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26)) |
                    (*(sptr - wpls27)) |
                    (*(sptr - wpls28));
        }
    }
}

static void
ferode_3_118(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls29)) &
                    (*(sptr - wpls28)) &
                    (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26)) &
                    (*(sptr + wpls27)) &
                    (*(sptr + wpls28));
        }
    }
}

static void
fdilate_3_119(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls29)) |
                    (*(sptr + wpls28)) |
                    (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26)) |
                    (*(sptr - wpls27)) |
                    (*(sptr - wpls28)) |
                    (*(sptr - wpls29));
        }
    }
}

static void
ferode_3_119(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls29)) &
                    (*(sptr - wpls28)) &
                    (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26)) &
                    (*(sptr + wpls27)) &
                    (*(sptr + wpls28)) &
                    (*(sptr + wpls29));
        }
    }
}

static void
fdilate_3_120(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29, wpls30;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    wpls30 = 30 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls30)) |
                    (*(sptr + wpls29)) |
                    (*(sptr + wpls28)) |
                    (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26)) |
                    (*(sptr - wpls27)) |
                    (*(sptr - wpls28)) |
                    (*(sptr - wpls29));
        }
    }
}

static void
ferode_3_120(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29, wpls30;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    wpls30 = 30 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls30)) &
                    (*(sptr - wpls29)) &
                    (*(sptr - wpls28)) &
                    (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26)) &
                    (*(sptr + wpls27)) &
                    (*(sptr + wpls28)) &
                    (*(sptr + wpls29));
        }
    }
}

static void
fdilate_3_121(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29, wpls30;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    wpls30 = 30 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls30)) |
                    (*(sptr + wpls29)) |
                    (*(sptr + wpls28)) |
                    (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26)) |
                    (*(sptr - wpls27)) |
                    (*(sptr - wpls28)) |
                    (*(sptr - wpls29)) |
                    (*(sptr - wpls30));
        }
    }
}

static void
ferode_3_121(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29, wpls30;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    wpls30 = 30 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls30)) &
                    (*(sptr - wpls29)) &
                    (*(sptr - wpls28)) &
                    (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26)) &
                    (*(sptr + wpls27)) &
                    (*(sptr + wpls28)) &
                    (*(sptr + wpls29)) &
                    (*(sptr + wpls30));
        }
    }
}

static void
fdilate_3_122(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29, wpls30, wpls31;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    wpls30 = 30 * wpls;
    wpls31 = 31 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls31)) |
                    (*(sptr + wpls30)) |
                    (*(sptr + wpls29)) |
                    (*(sptr + wpls28)) |
                    (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26)) |
                    (*(sptr - wpls27)) |
                    (*(sptr - wpls28)) |
                    (*(sptr - wpls29)) |
                    (*(sptr - wpls30));
        }
    }
}

static void
ferode_3_122(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29, wpls30, wpls31;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    wpls30 = 30 * wpls;
    wpls31 = 31 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls31)) &
                    (*(sptr - wpls30)) &
                    (*(sptr - wpls29)) &
                    (*(sptr - wpls28)) &
                    (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26)) &
                    (*(sptr + wpls27)) &
                    (*(sptr + wpls28)) &
                    (*(sptr + wpls29)) &
                    (*(sptr + wpls30));
        }
    }
}

static void
fdilate_3_123(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29, wpls30, wpls31;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    wpls30 = 30 * wpls;
    wpls31 = 31 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr + wpls31)) |
                    (*(sptr + wpls30)) |
                    (*(sptr + wpls29)) |
                    (*(sptr + wpls28)) |
                    (*(sptr + wpls27)) |
                    (*(sptr + wpls26)) |
                    (*(sptr + wpls25)) |
                    (*(sptr + wpls24)) |
                    (*(sptr + wpls23)) |
                    (*(sptr + wpls22)) |
                    (*(sptr + wpls21)) |
                    (*(sptr + wpls20)) |
                    (*(sptr + wpls19)) |
                    (*(sptr + wpls18)) |
                    (*(sptr + wpls17)) |
                    (*(sptr + wpls16)) |
                    (*(sptr + wpls15)) |
                    (*(sptr + wpls14)) |
                    (*(sptr + wpls13)) |
                    (*(sptr + wpls12)) |
                    (*(sptr + wpls11)) |
                    (*(sptr + wpls10)) |
                    (*(sptr + wpls9)) |
                    (*(sptr + wpls8)) |
                    (*(sptr + wpls7)) |
                    (*(sptr + wpls6)) |
                    (*(sptr + wpls5)) |
                    (*(sptr + wpls4)) |
                    (*(sptr + wpls3)) |
                    (*(sptr + wpls2)) |
                    (*(sptr + wpls)) |
                    (*sptr) |
                    (*(sptr - wpls)) |
                    (*(sptr - wpls2)) |
                    (*(sptr - wpls3)) |
                    (*(sptr - wpls4)) |
                    (*(sptr - wpls5)) |
                    (*(sptr - wpls6)) |
                    (*(sptr - wpls7)) |
                    (*(sptr - wpls8)) |
                    (*(sptr - wpls9)) |
                    (*(sptr - wpls10)) |
                    (*(sptr - wpls11)) |
                    (*(sptr - wpls12)) |
                    (*(sptr - wpls13)) |
                    (*(sptr - wpls14)) |
                    (*(sptr - wpls15)) |
                    (*(sptr - wpls16)) |
                    (*(sptr - wpls17)) |
                    (*(sptr - wpls18)) |
                    (*(sptr - wpls19)) |
                    (*(sptr - wpls20)) |
                    (*(sptr - wpls21)) |
                    (*(sptr - wpls22)) |
                    (*(sptr - wpls23)) |
                    (*(sptr - wpls24)) |
                    (*(sptr - wpls25)) |
                    (*(sptr - wpls26)) |
                    (*(sptr - wpls27)) |
                    (*(sptr - wpls28)) |
                    (*(sptr - wpls29)) |
                    (*(sptr - wpls30)) |
                    (*(sptr - wpls31));
        }
    }
}

static void
ferode_3_123(l_uint32  *datad,
            l_int32    w,
            l_int32    h,
            l_int32    wpld,
            l_uint32  *datas,
            l_int32    wpls)
{
l_int32   i;
l_int32   j, pwpls;
l_uint32  *sptr, *dptr;
l_int32             wpls2, wpls3, wpls4;
l_int32             wpls5, wpls6, wpls7, wpls8;
l_int32             wpls9, wpls10, wpls11, wpls12;
l_int32             wpls13, wpls14, wpls15, wpls16;
l_int32             wpls17, wpls18, wpls19, wpls20;
l_int32             wpls21, wpls22, wpls23, wpls24;
l_int32             wpls25, wpls26, wpls27, wpls28;
l_int32             wpls29, wpls30, wpls31;

    wpls2 = 2 * wpls;
    wpls3 = 3 * wpls;
    wpls4 = 4 * wpls;
    wpls5 = 5 * wpls;
    wpls6 = 6 * wpls;
    wpls7 = 7 * wpls;
    wpls8 = 8 * wpls;
    wpls9 = 9 * wpls;
    wpls10 = 10 * wpls;
    wpls11 = 11 * wpls;
    wpls12 = 12 * wpls;
    wpls13 = 13 * wpls;
    wpls14 = 14 * wpls;
    wpls15 = 15 * wpls;
    wpls16 = 16 * wpls;
    wpls17 = 17 * wpls;
    wpls18 = 18 * wpls;
    wpls19 = 19 * wpls;
    wpls20 = 20 * wpls;
    wpls21 = 21 * wpls;
    wpls22 = 22 * wpls;
    wpls23 = 23 * wpls;
    wpls24 = 24 * wpls;
    wpls25 = 25 * wpls;
    wpls26 = 26 * wpls;
    wpls27 = 27 * wpls;
    wpls28 = 28 * wpls;
    wpls29 = 29 * wpls;
    wpls30 = 30 * wpls;
    wpls31 = 31 * wpls;
    pwpls = (l_uint32)(w + 31) / 32;  /* proper wpl of src */

    for (i = 0; i < h; i++) {
        sptr = datas + i * wpls;
        dptr = datad + i * wpld;
        for (j = 0; j < pwpls; j++, sptr++, dptr++) {
            *dptr = (*(sptr - wpls31)) &
                    (*(sptr - wpls30)) &
                    (*(sptr - wpls29)) &
                    (*(sptr - wpls28)) &
                    (*(sptr - wpls27)) &
                    (*(sptr - wpls26)) &
                    (*(sptr - wpls25)) &
                    (*(sptr - wpls24)) &
                    (*(sptr - wpls23)) &
                    (*(sptr - wpls22)) &
                    (*(sptr - wpls21)) &
                    (*(sptr - wpls20)) &
                    (*(sptr - wpls19)) &
                    (*(sptr - wpls18)) &
                    (*(sptr - wpls17)) &
                    (*(sptr - wpls16)) &
                    (*(sptr - wpls15)) &
                    (*(sptr - wpls14)) &
                    (*(sptr - wpls13)) &
                    (*(sptr - wpls12)) &
                    (*(sptr - wpls11)) &
                    (*(sptr - wpls10)) &
                    (*(sptr - wpls9)) &
                    (*(sptr - wpls8)) &
                    (*(sptr - wpls7)) &
                    (*(sptr - wpls6)) &
                    (*(sptr - wpls5)) &
                    (*(sptr - wpls4)) &
                    (*(sptr - wpls3)) &
                    (*(sptr - wpls2)) &
                    (*(sptr - wpls)) &
                    (*sptr) &
                    (*(sptr + wpls)) &
                    (*(sptr + wpls2)) &
                    (*(sptr + wpls3)) &
                    (*(sptr + wpls4)) &
                    (*(sptr + wpls5)) &
                    (*(sptr + wpls6)) &
                    (*(sptr + wpls7)) &
                    (*(sptr + wpls8)) &
                    (*(sptr + wpls9)) &
                    (*(sptr + wpls10)) &
                    (*(sptr + wpls11)) &
                    (*(sptr + wpls12)) &
                    (*(sptr + wpls13)) &
                    (*(sptr + wpls14)) &
                    (*(sptr + wpls15)) &
                    (*(sptr + wpls16)) &
                    (*(sptr + wpls17)) &
                    (*(sptr + wpls18)) &
                    (*(sptr + wpls19)) &
                    (*(sptr + wpls20)) &
                    (*(sptr + wpls21)) &
                    (*(sptr + wpls22)) &
                    (*(sptr + wpls23)) &
                    (*(sptr + wpls24)) &
                    (*(sptr + wpls25)) &
                    (*(sptr + wpls26)) &
                    (*(sptr + wpls27)) &
                    (*(sptr + wpls28)) &
                    (*(sptr + wpls29)) &
                    (*(sptr + wpls30)) &
                    (*(sptr + wpls31));
        }
    }
}

