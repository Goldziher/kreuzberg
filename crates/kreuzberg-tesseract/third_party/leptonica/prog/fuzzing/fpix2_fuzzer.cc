#include "leptfuzz.h"

//static void MakePtas(l_int32 i, PTA **pptas, PTA **pptad);

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
    if(size<3) return 0;
 
    leptSetStdNullHandler();

    PIX *tmp_pixs = pixReadMemSpix(data, size);
    if(tmp_pixs == NULL) return 0;

    DPIX *dpix_payload = pixConvertToDPix(tmp_pixs, 1);
    if(dpix_payload == NULL) {
        pixDestroy(&tmp_pixs);
        return 0;
    }
    
    FPIX *fpix_payload = dpixConvertToFPix(dpix_payload);
    if(fpix_payload == NULL) {
        pixDestroy(&tmp_pixs);
        dpixDestroy(&dpix_payload);
        return 0;
    }

    DPIX *dpix_copy1 = dpixCopy(dpix_payload);
    dpixAddMultConstant(dpix_copy1, 1.0, 1.2);
    dpixDestroy(&dpix_copy1);

    DPIX *dpix_copy2 = dpixCopy(dpix_payload);
    FPIX *fpixs1 = dpixConvertToFPix(dpix_copy2);
    fpixDestroy(&fpixs1);
    dpixDestroy(&dpix_copy2);

    DPIX *dpix_copy3 = dpixCopy(dpix_payload);
    PIX *pix1 = dpixConvertToPix(dpix_copy3, 8, L_CLIP_TO_ZERO, 0);
    pixDestroy(&pix1);
    dpixDestroy(&dpix_copy3);

    l_float64 l_f1;
    l_int32 l_i1;
    l_int32 l_i2;
    DPIX *dpix_copy4 = dpixCopy(dpix_payload);
    dpixGetMax(dpix_copy4, &l_f1, &l_i1, &l_i2);
    dpixDestroy(&dpix_copy4);

    l_float64 l_f2;
    l_int32 l_i3;
    l_int32 l_i4;
    DPIX *dpix_copy5 = dpixCopy(dpix_payload);
    dpixGetMin(dpix_copy5, &l_f2, &l_i3, &l_i4);
    dpixDestroy(&dpix_copy5);
 
    DPIX *dpix1 = dpixCreate(300, 300);
    DPIX *dpix2 = dpixCreate(300, 300);
    DPIX *dpix_copy6 = dpixCopy(dpix_payload);
    dpixLinearCombination(dpix_copy6, dpix_copy6, dpix2, 1.1, 1.2);
    dpixDestroy(&dpix1);
    dpixDestroy(&dpix2);
    dpixDestroy(&dpix_copy6);
    
    DPIX *dpix_copy7 = dpixCopy(dpix_payload);
    DPIX *dpix3 = dpixScaleByInteger(dpix_copy7, 1);
    dpixDestroy(&dpix3);
    dpixDestroy(&dpix_copy7);

    DPIX *dpix_copy8 = dpixCopy(dpix_payload);
    dpixSetAllArbitrary(dpix_copy8, 1.1);
    dpixDestroy(&dpix_copy8);
    
    FPIX *fpix_copy1 = fpixCopy(fpix_payload);
    FPIX *fpix2 = fpixAddContinuedBorder(fpix_copy1, 1, 1, 1, 1);
    fpixDestroy(&fpix_copy1);
    fpixDestroy(&fpix2);
    
    PTA *pta1 = ptaCreate(0);
    PTA *pta2 = ptaCreate(0);
    FPIX *fpix_copy92 = fpixCopy(fpix_payload);
    FPIX *fpix3 = fpixAffinePta(fpix_copy92, pta1, pta2, 1, 0);
    fpixDestroy(&fpix3);
    fpixDestroy(&fpix_copy92);
    ptaDestroy(&pta1);
    ptaDestroy(&pta2);

    FPIX *fpix_copy2 = fpixCopy(fpix_payload);
    DPIX *dpix_return1 = fpixConvertToDPix(fpix_copy2);
    fpixDestroy(&fpix_copy2);
    dpixDestroy(&dpix_return1);
    
    FPIX *fpix5 = fpixCreate(300, 300);
    FPIX *fpix_copy3 = fpixCopy(fpix_payload);
    fpixLinearCombination(fpix_copy3, fpix_copy3, fpix5, 1.1, 1.1);
    fpixDestroy(&fpix_copy3);
    fpixDestroy(&fpix5);

    PTA *ptas, *ptad;
    ptas = ptaCreate(0);
    ptad = ptaCreate(0);
    FPIX *fpix_copy4 = fpixCopy(fpix_payload);
    FPIX *fpix_return2 = fpixProjectivePta(fpix_copy4, ptas, ptad, 200, 0.0);
    fpixDestroy(&fpix_return2);
    fpixDestroy(&fpix_copy4);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);

    pixDestroy(&tmp_pixs);
    dpixDestroy(&dpix_payload);
    fpixDestroy(&fpix_payload);
    return 0;
}
