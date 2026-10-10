// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_FORMFILL_H_
#define PUBLIC_FPDF_FORMFILL_H_

// clang-format off
// NOLINTNEXTLINE(build/include_directory)
#include "fpdfview.h"

// These values are return values for a public API, so should not be changed
// other than the count when adding new values.
#define FORMTYPE_NONE 0            // Document contains no forms
#define FORMTYPE_ACRO_FORM 1       // Forms are specified using AcroForm spec
#define FORMTYPE_XFA_FULL 2        // Forms are specified using entire XFA spec
#define FORMTYPE_XFA_FOREGROUND 3  // Forms are specified using the XFAF subset
                                   // of XFA spec
#define FORMTYPE_COUNT 4           // The number of form types

#define JSPLATFORM_ALERT_BUTTON_OK 0           // OK button
#define JSPLATFORM_ALERT_BUTTON_OKCANCEL 1     // OK & Cancel buttons
#define JSPLATFORM_ALERT_BUTTON_YESNO 2        // Yes & No buttons
#define JSPLATFORM_ALERT_BUTTON_YESNOCANCEL 3  // Yes, No & Cancel buttons
#define JSPLATFORM_ALERT_BUTTON_DEFAULT JSPLATFORM_ALERT_BUTTON_OK

#define JSPLATFORM_ALERT_ICON_ERROR 0     // Error
#define JSPLATFORM_ALERT_ICON_WARNING 1   // Warning
#define JSPLATFORM_ALERT_ICON_QUESTION 2  // Question
#define JSPLATFORM_ALERT_ICON_STATUS 3    // Status
#define JSPLATFORM_ALERT_ICON_ASTERISK 4  // Asterisk
#define JSPLATFORM_ALERT_ICON_DEFAULT JSPLATFORM_ALERT_ICON_ERROR

#define JSPLATFORM_ALERT_RETURN_OK 1      // OK
#define JSPLATFORM_ALERT_RETURN_CANCEL 2  // Cancel
#define JSPLATFORM_ALERT_RETURN_NO 3      // No
#define JSPLATFORM_ALERT_RETURN_YES 4     // Yes

#define JSPLATFORM_BEEP_ERROR 0           // Error
#define JSPLATFORM_BEEP_WARNING 1         // Warning
#define JSPLATFORM_BEEP_QUESTION 2        // Question
#define JSPLATFORM_BEEP_STATUS 3          // Status
#define JSPLATFORM_BEEP_DEFAULT 4         // Default

// Exported Functions
#ifdef __cplusplus
extern "C" {
#endif

typedef struct _IPDF_JsPlatform {
  // Version number of the interface. Currently must be 2.
  int version;

  // Version 1.

  // Method: app_alert
  //       Pop up a dialog to show warning or hint.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       Msg         -   A string containing the message to be displayed.
  //       Title       -   The title of the dialog.
  //       Type        -   The type of button group, one of the
  //                       JSPLATFORM_ALERT_BUTTON_* values above.
  //       nIcon       -   The type of the icon, one of the
  //                       JSPLATFORM_ALERT_ICON_* above.
  // Return Value:
  //       Option selected by user in dialogue, one of the
  //       JSPLATFORM_ALERT_RETURN_* values above.
  int (*app_alert)(struct _IPDF_JsPlatform* pThis,
                   FPDF_WIDESTRING Msg,
                   FPDF_WIDESTRING Title,
                   int Type,
                   int Icon);

  // Method: app_beep
  //       Causes the system to play a sound.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself
  //       nType       -   The sound type, see JSPLATFORM_BEEP_TYPE_*
  //                       above.
  // Return Value:
  //       None
  void (*app_beep)(struct _IPDF_JsPlatform* pThis, int nType);

  // Method: app_response
  //       Displays a dialog box containing a question and an entry field for
  //       the user to reply to the question.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself
  //       Question    -   The question to be posed to the user.
  //       Title       -   The title of the dialog box.
  //       Default     -   A default value for the answer to the question. If
  //                       not specified, no default value is presented.
  //       cLabel      -   A short string to appear in front of and on the
  //                       same line as the edit text field.
  //       bPassword   -   If true, indicates that the user's response should
  //                       be shown as asterisks (*) or bullets (?) to mask
  //                       the response, which might be sensitive information.
  //       response    -   A string buffer allocated by PDFium, to receive the
  //                       user's response.
  //       length      -   The length of the buffer in bytes. Currently, it is
  //                       always 2048.
  // Return Value:
  //       Number of bytes the complete user input would actually require, not
  //       including trailing zeros, regardless of the value of the length
  //       parameter or the presence of the response buffer.
  // Comments:
  //       No matter on what platform, the response buffer should be always
  //       written using UTF-16LE encoding. If a response buffer is
  //       present and the size of the user input exceeds the capacity of the
  //       buffer as specified by the length parameter, only the
  //       first "length" bytes of the user input are to be written to the
  //       buffer.
  int (*app_response)(struct _IPDF_JsPlatform* pThis,
                      FPDF_WIDESTRING Question,
                      FPDF_WIDESTRING Title,
                      FPDF_WIDESTRING Default,
                      FPDF_WIDESTRING cLabel,
                      FPDF_BOOL bPassword,
                      void* response,
                      int length);

  // Method: Doc_getFilePath
  //       Get the file path of the current document.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself
  //       filePath    -   The string buffer to receive the file path. Can
  //                       be NULL.
  //       length      -   The length of the buffer, number of bytes. Can
  //                       be 0.
  // Return Value:
  //       Number of bytes the filePath consumes, including trailing zeros.
  // Comments:
  //       The filePath should always be provided in the local encoding.
  //       The return value always indicated number of bytes required for
  //       the buffer, even when there is no buffer specified, or the buffer
  //       size is less than required. In this case, the buffer will not
  //       be modified.
  int (*Doc_getFilePath)(struct _IPDF_JsPlatform* pThis,
                         void* filePath,
                         int length);

  // Method: Doc_mail
  //       Mails the data buffer as an attachment to all recipients, with or
  //       without user interaction.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself
  //       mailData    -   Pointer to the data buffer to be sent. Can be NULL.
  //       length      -   The size,in bytes, of the buffer pointed by
  //                       mailData parameter. Can be 0.
  //       bUI         -   If true, the rest of the parameters are used in a
  //                       compose-new-message window that is displayed to the
  //                       user. If false, the cTo parameter is required and
  //                       all others are optional.
  //       To          -   A semicolon-delimited list of recipients for the
  //                       message.
  //       Subject     -   The subject of the message. The length limit is
  //                       64 KB.
  //       CC          -   A semicolon-delimited list of CC recipients for
  //                       the message.
  //       BCC         -   A semicolon-delimited list of BCC recipients for
  //                       the message.
  //       Msg         -   The content of the message. The length limit is
  //                       64 KB.
  // Return Value:
  //       None.
  // Comments:
  //       If the parameter mailData is NULL or length is 0, the current
  //       document will be mailed as an attachment to all recipients.
  void (*Doc_mail)(struct _IPDF_JsPlatform* pThis,
                   void* mailData,
                   int length,
                   FPDF_BOOL bUI,
                   FPDF_WIDESTRING To,
                   FPDF_WIDESTRING Subject,
                   FPDF_WIDESTRING CC,
                   FPDF_WIDESTRING BCC,
                   FPDF_WIDESTRING Msg);

  // Method: Doc_print
  //       Prints all or a specific number of pages of the document.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis         -   Pointer to the interface structure itself.
  //       bUI           -   If true, will cause a UI to be presented to the
  //                         user to obtain printing information and confirm
  //                         the action.
  //       nStart        -   A 0-based index that defines the start of an
  //                         inclusive range of pages.
  //       nEnd          -   A 0-based index that defines the end of an
  //                         inclusive page range.
  //       bSilent       -   If true, suppresses the cancel dialog box while
  //                         the document is printing. The default is false.
  //       bShrinkToFit  -   If true, the page is shrunk (if necessary) to
  //                         fit within the imageable area of the printed page.
  //       bPrintAsImage -   If true, print pages as an image.
  //       bReverse      -   If true, print from nEnd to nStart.
  //       bAnnotations  -   If true (the default), annotations are
  //                         printed.
  // Return Value:
  //       None.
  void (*Doc_print)(struct _IPDF_JsPlatform* pThis,
                    FPDF_BOOL bUI,
                    int nStart,
                    int nEnd,
                    FPDF_BOOL bSilent,
                    FPDF_BOOL bShrinkToFit,
                    FPDF_BOOL bPrintAsImage,
                    FPDF_BOOL bReverse,
                    FPDF_BOOL bAnnotations);

  // Method: Doc_submitForm
  //       Send the form data to a specified URL.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself
  //       formData    -   Pointer to the data buffer to be sent.
  //       length      -   The size,in bytes, of the buffer pointed by
  //                       formData parameter.
  //       URL         -   The URL to send to.
  // Return Value:
  //       None.
  void (*Doc_submitForm)(struct _IPDF_JsPlatform* pThis,
                         void* formData,
                         int length,
                         FPDF_WIDESTRING URL);

  // Method: Doc_gotoPage
  //       Jump to a specified page.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself
  //       nPageNum    -   The specified page number, zero for the first page.
  // Return Value:
  //       None.
  void (*Doc_gotoPage)(struct _IPDF_JsPlatform* pThis, int nPageNum);

  // Method: Field_browse
  //       Show a file selection dialog, and return the selected file path.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       filePath    -   Pointer to the data buffer to receive the file
  //                       path. Can be NULL.
  //       length      -   The length of the buffer, in bytes. Can be 0.
  // Return Value:
  //       Number of bytes the filePath consumes, including trailing zeros.
  // Comments:
  //       The filePath should always be provided in local encoding.
  int (*Field_browse)(struct _IPDF_JsPlatform* pThis,
                      void* filePath,
                      int length);

  // Pointer for embedder-specific data. Unused by PDFium, and despite
  // its name, can be any data the embedder desires, though traditionally
  // a FPDF_FORMFILLINFO interface.
  void* m_pFormfillinfo;

  // Version 2.

  void* m_isolate;               // Unused in v3, retain for compatibility.
  unsigned int m_v8EmbedderSlot; // Unused in v3, retain for compatibility.

  // Version 3.
  // Version 3 moves m_Isolate and m_v8EmbedderSlot to FPDF_LIBRARY_CONFIG.
} IPDF_JSPLATFORM;

// Flags for Cursor type
#define FXCT_ARROW 0
#define FXCT_NESW 1
#define FXCT_NWSE 2
#define FXCT_VBEAM 3
#define FXCT_HBEAM 4
#define FXCT_HAND 5

// Function signature for the callback function passed to the FFI_SetTimer
// method.
// Parameters:
//          idEvent     -   Identifier of the timer.
// Return value:
//          None.
typedef void (*TimerCallback)(int idEvent);

// Declares of a struct type to the local system time.
typedef struct _FPDF_SYSTEMTIME {
  unsigned short wYear;         // years since 1900
  unsigned short wMonth;        // months since January - [0,11]
  unsigned short wDayOfWeek;    // days since Sunday - [0,6]
  unsigned short wDay;          // day of the month - [1,31]
  unsigned short wHour;         // hours since midnight - [0,23]
  unsigned short wMinute;       // minutes after the hour - [0,59]
  unsigned short wSecond;       // seconds after the minute - [0,59]
  unsigned short wMilliseconds; // milliseconds after the second - [0,999]
} FPDF_SYSTEMTIME;

#ifdef PDF_ENABLE_XFA

// Pageview event flags
#define FXFA_PAGEVIEWEVENT_POSTADDED 1    // After a new pageview is added.
#define FXFA_PAGEVIEWEVENT_POSTREMOVED 3  // After a pageview is removed.

// Definitions for Right Context Menu Features Of XFA Fields
#define FXFA_MENU_COPY 1
#define FXFA_MENU_CUT 2
#define FXFA_MENU_SELECTALL 4
#define FXFA_MENU_UNDO 8
#define FXFA_MENU_REDO 16
#define FXFA_MENU_PASTE 32

// Definitions for File Type.
#define FXFA_SAVEAS_XML 1
#define FXFA_SAVEAS_XDP 2

#endif  // PDF_ENABLE_XFA

typedef struct _FPDF_FORMFILLINFO {
  // Version number of the interface.
  // Version 1 contains stable interfaces. Version 2 has additional
  // experimental interfaces.
  // When PDFium is built without the XFA module, version can be 1 or 2.
  // With version 1, only stable interfaces are called. With version 2,
  // additional experimental interfaces are also called.
  // When PDFium is built with the XFA module, version must be 2.
  // All the XFA related interfaces are experimental. If PDFium is built with
  // the XFA module and version 1 then none of the XFA related interfaces
  // would be called. When PDFium is built with XFA module then the version
  // must be 2.
  int version;

  // Version 1.

  // Method: Release
  //       Give the implementation a chance to release any resources after the
  //       interface is no longer used.
  // Interface Version:
  //       1
  // Implementation Required:
  //       No
  // Comments:
  //       Called by PDFium during the final cleanup process.
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself
  // Return Value:
  //       None
  void (*Release)(struct _FPDF_FORMFILLINFO* pThis);

  // Method: FFI_Invalidate
  //       Invalidate the client area within the specified rectangle.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       page        -   Handle to the page. Returned by FPDF_LoadPage().
  //       left        -   Left position of the client area in PDF page
  //                       coordinates.
  //       top         -   Top position of the client area in PDF page
  //                       coordinates.
  //       right       -   Right position of the client area in PDF page
  //                       coordinates.
  //       bottom      -   Bottom position of the client area in PDF page
  //                       coordinates.
  // Return Value:
  //       None.
  // Comments:
  //       All positions are measured in PDF "user space".
  //       Implementation should call FPDF_RenderPageBitmap() for repainting
  //       the specified page area.
  void (*FFI_Invalidate)(struct _FPDF_FORMFILLINFO* pThis,
                         FPDF_PAGE page,
                         double left,
                         double top,
                         double right,
                         double bottom);

  // Method: FFI_OutputSelectedRect
  //       When the user selects text in form fields with the mouse, this
  //       callback function will be invoked with the selected areas.
  // Interface Version:
  //       1
  // Implementation Required:
  //       No
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       page        -   Handle to the page. Returned by FPDF_LoadPage()/
  //       left        -   Left position of the client area in PDF page
  //                       coordinates.
  //       top         -   Top position of the client area in PDF page
  //                       coordinates.
  //       right       -   Right position of the client area in PDF page
  //                       coordinates.
  //       bottom      -   Bottom position of the client area in PDF page
  //                       coordinates.
  // Return Value:
  //       None.
  // Comments:
  //       This callback function is useful for implementing special text
  //       selection effects. An implementation should first record the
  //       returned rectangles, then draw them one by one during the next
  //       painting period. Lastly, it should remove all the recorded
  //       rectangles when finished painting.
  void (*FFI_OutputSelectedRect)(struct _FPDF_FORMFILLINFO* pThis,
                                 FPDF_PAGE page,
                                 double left,
                                 double top,
                                 double right,
                                 double bottom);

  // Method: FFI_SetCursor
  //       Set the Cursor shape.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       nCursorType -   Cursor type, see Flags for Cursor type for details.
  // Return value:
  //       None.
  void (*FFI_SetCursor)(struct _FPDF_FORMFILLINFO* pThis, int nCursorType);

  // Method: FFI_SetTimer
  //       This method installs a system timer. An interval value is specified,
  //       and every time that interval elapses, the system must call into the
  //       callback function with the timer ID as returned by this function.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       uElapse     -   Specifies the time-out value, in milliseconds.
  //       lpTimerFunc -   A pointer to the callback function-TimerCallback.
  // Return value:
  //       The timer identifier of the new timer if the function is successful.
  //       An application passes this value to the FFI_KillTimer method to kill
  //       the timer. Nonzero if it is successful; otherwise, it is zero.
  int (*FFI_SetTimer)(struct _FPDF_FORMFILLINFO* pThis,
                      int uElapse,
                      TimerCallback lpTimerFunc);

  // Method: FFI_KillTimer
  //       This method uninstalls a system timer, as set by an earlier call to
  //       FFI_SetTimer.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       nTimerID    -   The timer ID returned by FFI_SetTimer function.
  // Return value:
  //       None.
  void (*FFI_KillTimer)(struct _FPDF_FORMFILLINFO* pThis, int nTimerID);

  // Method: FFI_GetLocalTime
  //       This method receives the current local time on the system.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  // Return value:
  //       The local time. See FPDF_SYSTEMTIME above for details.
  // Note: Unused.
  FPDF_SYSTEMTIME (*FFI_GetLocalTime)(struct _FPDF_FORMFILLINFO* pThis);

  // Method: FFI_OnChange
  //       This method will be invoked to notify the implementation when the
  //       value of any FormField on the document had been changed.
  // Interface Version:
  //       1
  // Implementation Required:
  //       no
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  // Return value:
  //       None.
  void (*FFI_OnChange)(struct _FPDF_FORMFILLINFO* pThis);

  // Method: FFI_GetPage
  //       This method receives the page handle associated with a specified
  //       page index.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       document    -   Handle to document. Returned by FPDF_LoadDocument().
  //       nPageIndex  -   Index number of the page. 0 for the first page.
  // Return value:
  //       Handle to the page, as previously returned to the implementation by
  //       FPDF_LoadPage().
  // Comments:
  //       The implementation is expected to keep track of the page handles it
  //       receives from PDFium, and their mappings to page numbers. In some
  //       cases, the document-level JavaScript action may refer to a page
  //       which hadn't been loaded yet. To successfully run the Javascript
  //       action, the implementation needs to load the page.
  FPDF_PAGE (*FFI_GetPage)(struct _FPDF_FORMFILLINFO* pThis,
                           FPDF_DOCUMENT document,
                           int nPageIndex);

  // Method: FFI_GetCurrentPage
  //       This method receives the handle to the current page.
  // Interface Version:
  //       1
  // Implementation Required:
  //       Yes when V8 support is present, otherwise unused.
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       document    -   Handle to document. Returned by FPDF_LoadDocument().
  // Return value:
  //       Handle to the page. Returned by FPDF_LoadPage().
  // Comments:
  //       PDFium doesn't keep keep track of the "current page" (e.g. the one
  //       that is most visible on screen), so it must ask the embedder for
  //       this information.
  FPDF_PAGE (*FFI_GetCurrentPage)(struct _FPDF_FORMFILLINFO* pThis,
                                  FPDF_DOCUMENT document);

  // Method: FFI_GetRotation
  //       This method receives currently rotation of the page view.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis       -   Pointer to the interface structure itself.
  //       page        -   Handle to page, as returned by FPDF_LoadPage().
  // Return value:
  //       A number to indicate the page rotation in 90 degree increments
  //       in a clockwise direction:
  //         0 - 0 degrees
  //         1 - 90 degrees
  //         2 - 180 degrees
  //         3 - 270 degrees
  // Note: Unused.
  int (*FFI_GetRotation)(struct _FPDF_FORMFILLINFO* pThis, FPDF_PAGE page);

  // Method: FFI_ExecuteNamedAction
  //       This method will execute a named action.
  // Interface Version:
  //       1
  // Implementation Required:
  //       yes
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       namedAction     -   A byte string which indicates the named action,
  //                           terminated by 0.
  // Return value:
  //       None.
  // Comments:
  //       See ISO 32000-1:2008, section 12.6.4.11 for descriptions of the
  //       standard named actions, but note that a document may supply any
  //       name of its choosing.
  void (*FFI_ExecuteNamedAction)(struct _FPDF_FORMFILLINFO* pThis,
                                 FPDF_BYTESTRING namedAction);
  // Method: FFI_SetTextFieldFocus
  //       Called when a text field is getting or losing focus.
  // Interface Version:
  //       1
  // Implementation Required:
  //       no
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       value           -   The string value of the form field, in UTF-16LE
  //                           format.
  //       valueLen        -   The length of the string value. This is the
  //                           number of characters, not bytes.
  //       is_focus        -   True if the form field is getting focus, false
  //                           if the form field is losing focus.
  // Return value:
  //       None.
  // Comments:
  //       Only supports text fields and combobox fields.
  void (*FFI_SetTextFieldFocus)(struct _FPDF_FORMFILLINFO* pThis,
                                FPDF_WIDESTRING value,
                                FPDF_DWORD valueLen,
                                FPDF_BOOL is_focus);

  // Method: FFI_DoURIAction
  //       Ask the implementation to navigate to a uniform resource identifier.
  // Interface Version:
  //       1
  // Implementation Required:
  //       No
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       bsURI           -   A byte string which indicates the uniform
  //                           resource identifier, terminated by 0.
  // Return value:
  //       None.
  // Comments:
  //       If the embedder is version 2 or higher and have implementation for
  //       FFI_DoURIActionWithKeyboardModifier, then
  //       FFI_DoURIActionWithKeyboardModifier takes precedence over
  //       FFI_DoURIAction.
  //       See the URI actions description of <<PDF Reference, version 1.7>>
  //       for more details.
  void (*FFI_DoURIAction)(struct _FPDF_FORMFILLINFO* pThis,
                          FPDF_BYTESTRING bsURI);

  // Method: FFI_DoGoToAction
  //       This action changes the view to a specified destination.
  // Interface Version:
  //       1
  // Implementation Required:
  //       No
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       nPageIndex      -   The index of the PDF page.
  //       zoomMode        -   The zoom mode for viewing page. See below.
  //       fPosArray       -   The float array which carries the position info.
  //       sizeofArray     -   The size of float array.
  // PDFZoom values:
  //         - XYZ = 1
  //         - FITPAGE = 2
  //         - FITHORZ = 3
  //         - FITVERT = 4
  //         - FITRECT = 5
  //         - FITBBOX = 6
  //         - FITBHORZ = 7
  //         - FITBVERT = 8
  // Return value:
  //       None.
  // Comments:
  //       See the Destinations description of <<PDF Reference, version 1.7>>
  //       in 8.2.1 for more details.
  void (*FFI_DoGoToAction)(struct _FPDF_FORMFILLINFO* pThis,
                           int nPageIndex,
                           int zoomMode,
                           float* fPosArray,
                           int sizeofArray);

  // Pointer to IPDF_JSPLATFORM interface.
  // Unused if PDFium is built without V8 support. Otherwise, if NULL, then
  // JavaScript will be prevented from executing while rendering the document.
  IPDF_JSPLATFORM* m_pJsPlatform;

  // Version 2 - Experimental.

  // Whether the XFA module is disabled when built with the XFA module.
  // Interface Version:
  //       Ignored if |version| < 2.
  FPDF_BOOL xfa_disabled;

  // Method: FFI_DisplayCaret
  //       This method will show the caret at specified position.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       page            -   Handle to page. Returned by FPDF_LoadPage().
  //       left            -   Left position of the client area in PDF page
  //                           coordinates.
  //       top             -   Top position of the client area in PDF page
  //                           coordinates.
  //       right           -   Right position of the client area in PDF page
  //                           coordinates.
  //       bottom          -   Bottom position of the client area in PDF page
  //                           coordinates.
  // Return value:
  //       None.
  void (*FFI_DisplayCaret)(struct _FPDF_FORMFILLINFO* pThis,
                           FPDF_PAGE page,
                           FPDF_BOOL bVisible,
                           double left,
                           double top,
                           double right,
                           double bottom);

  // Method: FFI_GetCurrentPageIndex
  //       This method will get the current page index.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       document        -   Handle to document from FPDF_LoadDocument().
  // Return value:
  //       The index of current page.
  int (*FFI_GetCurrentPageIndex)(struct _FPDF_FORMFILLINFO* pThis,
                                 FPDF_DOCUMENT document);

  // Method: FFI_SetCurrentPage
  //       This method will set the current page.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       document        -   Handle to document from FPDF_LoadDocument().
  //       iCurPage        -   The index of the PDF page.
  // Return value:
  //       None.
  void (*FFI_SetCurrentPage)(struct _FPDF_FORMFILLINFO* pThis,
                             FPDF_DOCUMENT document,
                             int iCurPage);

 // Method: FFI_GotoURL
 //       This method will navigate to the specified URL.
 // Interface Version:
 //       Ignored if |version| < 2.
 // Implementation Required:
 //       Required for XFA, otherwise set to NULL.
 // Parameters:
 //       pThis            -   Pointer to the interface structure itself.
 //       document         -   Handle to document from FPDF_LoadDocument().
 //       wsURL            -   The string value of the URL, in UTF-16LE format.
 // Return value:
 //       None.
  void (*FFI_GotoURL)(struct _FPDF_FORMFILLINFO* pThis,
                      FPDF_DOCUMENT document,
                      FPDF_WIDESTRING wsURL);

  // Method: FFI_GetPageViewRect
  //       This method will get the current page view rectangle.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       page            -   Handle to page. Returned by FPDF_LoadPage().
  //       left            -   The pointer to receive left position of the page
  //                           view area in PDF page coordinates.
  //       top             -   The pointer to receive top position of the page
  //                           view area in PDF page coordinates.
  //       right           -   The pointer to receive right position of the
  //                           page view area in PDF page coordinates.
  //       bottom          -   The pointer to receive bottom position of the
  //                           page view area in PDF page coordinates.
  // Return value:
  //     None.
  void (*FFI_GetPageViewRect)(struct _FPDF_FORMFILLINFO* pThis,
                              FPDF_PAGE page,
                              double* left,
                              double* top,
                              double* right,
                              double* bottom);

  // Method: FFI_PageEvent
  //       This method fires when pages have been added to or deleted from
  //       the XFA document.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       page_count      -   The number of pages to be added or deleted.
  //       event_type      -   See FXFA_PAGEVIEWEVENT_* above.
  // Return value:
  //       None.
  // Comments:
  //       The pages to be added or deleted always start from the last page
  //       of document. This means that if parameter page_count is 2 and
  //       event type is FXFA_PAGEVIEWEVENT_POSTADDED, 2 new pages have been
  //       appended to the tail of document; If page_count is 2 and
  //       event type is FXFA_PAGEVIEWEVENT_POSTREMOVED, the last 2 pages
  //       have been deleted.
  void (*FFI_PageEvent)(struct _FPDF_FORMFILLINFO* pThis,
                        int page_count,
                        FPDF_DWORD event_type);

  // Method: FFI_PopupMenu
  //       This method will track the right context menu for XFA fields.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       page            -   Handle to page. Returned by FPDF_LoadPage().
  //       hWidget         -   Always null, exists for compatibility.
  //       menuFlag        -   The menu flags. Please refer to macro definition
  //                           of FXFA_MENU_XXX and this can be one or a
  //                           combination of these macros.
  //       x               -   X position of the client area in PDF page
  //                           coordinates.
  //       y               -   Y position of the client area in PDF page
  //                           coordinates.
  // Return value:
  //       TRUE indicates success; otherwise false.
  FPDF_BOOL (*FFI_PopupMenu)(struct _FPDF_FORMFILLINFO* pThis,
                             FPDF_PAGE page,
                             FPDF_WIDGET hWidget,
                             int menuFlag,
                             float x,
                             float y);

  // Method: FFI_OpenFile
  //       This method will open the specified file with the specified mode.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       fileFlag        -   The file flag. Please refer to macro definition
  //                           of FXFA_SAVEAS_XXX and use one of these macros.
  //       wsURL           -   The string value of the file URL, in UTF-16LE
  //                           format.
  //       mode            -   The mode for open file, e.g. "rb" or "wb".
  // Return value:
  //       The handle to FPDF_FILEHANDLER.
  FPDF_FILEHANDLER* (*FFI_OpenFile)(struct _FPDF_FORMFILLINFO* pThis,
                                    int fileFlag,
                                    FPDF_WIDESTRING wsURL,
                                    const char* mode);

  // Method: FFI_EmailTo
  //       This method will email the specified file stream to the specified
  //       contact.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       pFileHandler    -   Handle to the FPDF_FILEHANDLER.
  //       pTo             -   A semicolon-delimited list of recipients for the
  //                           message,in UTF-16LE format.
  //       pSubject        -   The subject of the message,in UTF-16LE format.
  //       pCC             -   A semicolon-delimited list of CC recipients for
  //                           the message,in UTF-16LE format.
  //       pBcc            -   A semicolon-delimited list of BCC recipients for
  //                           the message,in UTF-16LE format.
  //       pMsg            -   Pointer to the data buffer to be sent.Can be
  //                           NULL,in UTF-16LE format.
  // Return value:
  //       None.
  void (*FFI_EmailTo)(struct _FPDF_FORMFILLINFO* pThis,
                      FPDF_FILEHANDLER* fileHandler,
                      FPDF_WIDESTRING pTo,
                      FPDF_WIDESTRING pSubject,
                      FPDF_WIDESTRING pCC,
                      FPDF_WIDESTRING pBcc,
                      FPDF_WIDESTRING pMsg);

  // Method: FFI_UploadTo
  //       This method will upload the specified file stream to the
  //       specified URL.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       pFileHandler    -   Handle to the FPDF_FILEHANDLER.
  //       fileFlag        -   The file flag. Please refer to macro definition
  //                           of FXFA_SAVEAS_XXX and use one of these macros.
  //       uploadTo        -   Pointer to the URL path, in UTF-16LE format.
  // Return value:
  //       None.
  void (*FFI_UploadTo)(struct _FPDF_FORMFILLINFO* pThis,
                       FPDF_FILEHANDLER* fileHandler,
                       int fileFlag,
                       FPDF_WIDESTRING uploadTo);

  // Method: FFI_GetPlatform
  //       This method will get the current platform.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       platform        -   Pointer to the data buffer to receive the
  //                           platform,in UTF-16LE format. Can be NULL.
  //       length          -   The length of the buffer in bytes. Can be
  //                           0 to query the required size.
  // Return value:
  //       The length of the buffer, number of bytes.
  int (*FFI_GetPlatform)(struct _FPDF_FORMFILLINFO* pThis,
                         void* platform,
                         int length);

  // Method: FFI_GetLanguage
  //       This method will get the current language.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       language        -   Pointer to the data buffer to receive the
  //                           current language. Can be NULL.
  //       length          -   The length of the buffer in bytes. Can be
  //                           0 to query the required size.
  // Return value:
  //       The length of the buffer, number of bytes.
  int (*FFI_GetLanguage)(struct _FPDF_FORMFILLINFO* pThis,
                         void* language,
                         int length);

  // Method: FFI_DownloadFromURL
  //       This method will download the specified file from the URL.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       URL             -   The string value of the file URL, in UTF-16LE
  //                           format.
  // Return value:
  //       The handle to FPDF_FILEHANDLER.
  FPDF_FILEHANDLER* (*FFI_DownloadFromURL)(struct _FPDF_FORMFILLINFO* pThis,
                                           FPDF_WIDESTRING URL);
  // Method: FFI_PostRequestURL
  //       This method will post the request to the server URL.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       wsURL           -   The string value of the server URL, in UTF-16LE
  //                           format.
  //       wsData          -   The post data,in UTF-16LE format.
  //       wsContentType   -   The content type of the request data, in
  //                           UTF-16LE format.
  //       wsEncode        -   The encode type, in UTF-16LE format.
  //       wsHeader        -   The request header,in UTF-16LE format.
  //       response        -   Pointer to the FPDF_BSTR to receive the response
  //                           data from the server, in UTF-16LE format.
  // Return value:
  //       TRUE indicates success, otherwise FALSE.
  FPDF_BOOL (*FFI_PostRequestURL)(struct _FPDF_FORMFILLINFO* pThis,
                                  FPDF_WIDESTRING wsURL,
                                  FPDF_WIDESTRING wsData,
                                  FPDF_WIDESTRING wsContentType,
                                  FPDF_WIDESTRING wsEncode,
                                  FPDF_WIDESTRING wsHeader,
                                  FPDF_BSTR* response);

  // Method: FFI_PutRequestURL
  //       This method will put the request to the server URL.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       Required for XFA, otherwise set to NULL.
  // Parameters:
  //       pThis           -   Pointer to the interface structure itself.
  //       wsURL           -   The string value of the server URL, in UTF-16LE
  //                           format.
  //       wsData          -   The put data, in UTF-16LE format.
  //       wsEncode        -   The encode type, in UTR-16LE format.
  // Return value:
  //       TRUE indicates success, otherwise FALSE.
  FPDF_BOOL (*FFI_PutRequestURL)(struct _FPDF_FORMFILLINFO* pThis,
                                 FPDF_WIDESTRING wsURL,
                                 FPDF_WIDESTRING wsData,
                                 FPDF_WIDESTRING wsEncode);

  // Method: FFI_OnFocusChange
  //     Called when the focused annotation is updated.
  // Interface Version:
  //     Ignored if |version| < 2.
  // Implementation Required:
  //     No
  // Parameters:
  //     param           -   Pointer to the interface structure itself.
  //     annot           -   The focused annotation.
  //     page_index      -   Index number of the page which contains the
  //                         focused annotation. 0 for the first page.
  // Return value:
  //     None.
  // Comments:
  //     This callback function is useful for implementing any view based
  //     action such as scrolling the annotation rect into view. The
  //     embedder should not copy and store the annot as its scope is
  //     limited to this call only.
  void (*FFI_OnFocusChange)(struct _FPDF_FORMFILLINFO* param,
                            FPDF_ANNOTATION annot,
                            int page_index);

  // Method: FFI_DoURIActionWithKeyboardModifier
  //       Ask the implementation to navigate to a uniform resource identifier
  //       with the specified modifiers.
  // Interface Version:
  //       Ignored if |version| < 2.
  // Implementation Required:
  //       No
  // Parameters:
  //       param           -   Pointer to the interface structure itself.
  //       uri             -   A byte string which indicates the uniform
  //                           resource identifier, terminated by 0.
  //       modifiers       -   Keyboard modifier that indicates which of
  //                           the virtual keys are down, if any.
  // Return value:
  //       None.
  // Comments:
  //       If the embedder who is version 2 and does not implement this API,
  //       then a call will be redirected to FFI_DoURIAction.
  //       See the URI actions description of <<PDF Reference, version 1.7>>
  //       for more details.
  void(*FFI_DoURIActionWithKeyboardModifier)(struct _FPDF_FORMFILLINFO* param,
      FPDF_BYTESTRING uri,
      int modifiers);
} FPDF_FORMFILLINFO;

// Function: FPDFDOC_InitFormFillEnvironment
//       Initialize form fill environment.
// Parameters:
//       document        -   Handle to document from FPDF_LoadDocument().
//       formInfo        -   Pointer to a FPDF_FORMFILLINFO structure.
// Return Value:
//       Handle to the form fill module, or NULL on failure.
// Comments:
//       This function should be called before any form fill operation.
//       The FPDF_FORMFILLINFO passed in via |formInfo| must remain valid until
//       the returned FPDF_FORMHANDLE is closed.
FPDF_EXPORT FPDF_FORMHANDLE FPDF_CALLCONV
FPDFDOC_InitFormFillEnvironment(FPDF_DOCUMENT document,
                                FPDF_FORMFILLINFO* formInfo);

// Function: FPDFDOC_ExitFormFillEnvironment
//       Take ownership of |hHandle| and exit form fill environment.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
// Return Value:
//       None.
// Comments:
//       This function is a no-op when |hHandle| is null.
FPDF_EXPORT void FPDF_CALLCONV
FPDFDOC_ExitFormFillEnvironment(FPDF_FORMHANDLE hHandle);

// Function: FORM_OnAfterLoadPage
//       This method is required for implementing all the form related
//       functions. Should be invoked after user successfully loaded a
//       PDF page, and FPDFDOC_InitFormFillEnvironment() has been invoked.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
// Return Value:
//       None.
FPDF_EXPORT void FPDF_CALLCONV FORM_OnAfterLoadPage(FPDF_PAGE page,
                                                    FPDF_FORMHANDLE hHandle);

// Function: FORM_OnBeforeClosePage
//       This method is required for implementing all the form related
//       functions. Should be invoked before user closes the PDF page.
// Parameters:
//        page        -   Handle to the page, as returned by FPDF_LoadPage().
//        hHandle     -   Handle to the form fill module, as returned by
//                        FPDFDOC_InitFormFillEnvironment().
// Return Value:
//        None.
FPDF_EXPORT void FPDF_CALLCONV FORM_OnBeforeClosePage(FPDF_PAGE page,
                                                      FPDF_FORMHANDLE hHandle);

// Function: FORM_DoDocumentJSAction
//       This method is required for performing document-level JavaScript
//       actions. It should be invoked after the PDF document has been loaded.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
// Return Value:
//       None.
// Comments:
//       If there is document-level JavaScript action embedded in the
//       document, this method will execute the JavaScript action. Otherwise,
//       the method will do nothing.
FPDF_EXPORT void FPDF_CALLCONV
FORM_DoDocumentJSAction(FPDF_FORMHANDLE hHandle);

// Function: FORM_DoDocumentOpenAction
//       This method is required for performing open-action when the document
//       is opened.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
// Return Value:
//       None.
// Comments:
//       This method will do nothing if there are no open-actions embedded
//       in the document.
FPDF_EXPORT void FPDF_CALLCONV
FORM_DoDocumentOpenAction(FPDF_FORMHANDLE hHandle);

// Additional actions type of document:
//   WC, before closing document, JavaScript action.
//   WS, before saving document, JavaScript action.
//   DS, after saving document, JavaScript action.
//   WP, before printing document, JavaScript action.
//   DP, after printing document, JavaScript action.
#define FPDFDOC_AACTION_WC 0x10
#define FPDFDOC_AACTION_WS 0x11
#define FPDFDOC_AACTION_DS 0x12
#define FPDFDOC_AACTION_WP 0x13
#define FPDFDOC_AACTION_DP 0x14

// Function: FORM_DoDocumentAAction
//       This method is required for performing the document's
//       additional-action.
// Parameters:
//       hHandle     -   Handle to the form fill module. Returned by
//                       FPDFDOC_InitFormFillEnvironment.
//       aaType      -   The type of the additional-actions which defined
//                       above.
// Return Value:
//       None.
// Comments:
//       This method will do nothing if there is no document
//       additional-action corresponding to the specified |aaType|.
FPDF_EXPORT void FPDF_CALLCONV FORM_DoDocumentAAction(FPDF_FORMHANDLE hHandle,
                                                      int aaType);

// Additional-action types of page object:
//   OPEN (/O) -- An action to be performed when the page is opened
//   CLOSE (/C) -- An action to be performed when the page is closed
#define FPDFPAGE_AACTION_OPEN 0
#define FPDFPAGE_AACTION_CLOSE 1

// Function: FORM_DoPageAAction
//       This method is required for performing the page object's
//       additional-action when opened or closed.
// Parameters:
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       aaType      -   The type of the page object's additional-actions
//                       which defined above.
// Return Value:
//       None.
// Comments:
//       This method will do nothing if no additional-action corresponding
//       to the specified |aaType| exists.
FPDF_EXPORT void FPDF_CALLCONV FORM_DoPageAAction(FPDF_PAGE page,
                                                  FPDF_FORMHANDLE hHandle,
                                                  int aaType);

// Function: FORM_OnMouseMove
//       Call this member function when the mouse cursor moves.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       modifier    -   Indicates whether various virtual keys are down.
//       page_x      -   Specifies the x-coordinate of the cursor in PDF user
//                       space.
//       page_y      -   Specifies the y-coordinate of the cursor in PDF user
//                       space.
// Return Value:
//       True indicates success; otherwise false.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnMouseMove(FPDF_FORMHANDLE hHandle,
                                                     FPDF_PAGE page,
                                                     int modifier,
                                                     double page_x,
                                                     double page_y);

// Experimental API
// Function: FORM_OnMouseWheel
//       Call this member function when the user scrolls the mouse wheel.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       modifier    -   Indicates whether various virtual keys are down.
//       page_coord  -   Specifies the coordinates of the cursor in PDF user
//                       space.
//       delta_x     -   Specifies the amount of wheel movement on the x-axis,
//                       in units of platform-agnostic wheel deltas. Negative
//                       values mean left.
//       delta_y     -   Specifies the amount of wheel movement on the y-axis,
//                       in units of platform-agnostic wheel deltas. Negative
//                       values mean down.
// Return Value:
//       True indicates success; otherwise false.
// Comments:
//       For |delta_x| and |delta_y|, the caller must normalize
//       platform-specific wheel deltas. e.g. On Windows, a delta value of 240
//       for a WM_MOUSEWHEEL event normalizes to 2, since Windows defines
//       WHEEL_DELTA as 120.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnMouseWheel(
    FPDF_FORMHANDLE hHandle,
    FPDF_PAGE page,
    int modifier,
    const FS_POINTF* page_coord,
    int delta_x,
    int delta_y);

// Function: FORM_OnFocus
//       This function focuses the form annotation at a given point. If the
//       annotation at the point already has focus, nothing happens. If there
//       is no annotation at the point, removes form focus.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       modifier    -   Indicates whether various virtual keys are down.
//       page_x      -   Specifies the x-coordinate of the cursor in PDF user
//                       space.
//       page_y      -   Specifies the y-coordinate of the cursor in PDF user
//                       space.
// Return Value:
//       True if there is an annotation at the given point and it has focus.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnFocus(FPDF_FORMHANDLE hHandle,
                                                 FPDF_PAGE page,
                                                 int modifier,
                                                 double page_x,
                                                 double page_y);

// Function: FORM_OnLButtonDown
//       Call this member function when the user presses the left
//       mouse button.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       modifier    -   Indicates whether various virtual keys are down.
//       page_x      -   Specifies the x-coordinate of the cursor in PDF user
//                       space.
//       page_y      -   Specifies the y-coordinate of the cursor in PDF user
//                       space.
// Return Value:
//       True indicates success; otherwise false.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnLButtonDown(FPDF_FORMHANDLE hHandle,
                                                       FPDF_PAGE page,
                                                       int modifier,
                                                       double page_x,
                                                       double page_y);

// Function: FORM_OnRButtonDown
//       Same as above, execpt for the right mouse button.
// Comments:
//       At the present time, has no effect except in XFA builds, but is
//       included for the sake of symmetry.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnRButtonDown(FPDF_FORMHANDLE hHandle,
                                                       FPDF_PAGE page,
                                                       int modifier,
                                                       double page_x,
                                                       double page_y);
// Function: FORM_OnLButtonUp
//       Call this member function when the user releases the left
//       mouse button.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       modifier    -   Indicates whether various virtual keys are down.
//       page_x      -   Specifies the x-coordinate of the cursor in device.
//       page_y      -   Specifies the y-coordinate of the cursor in device.
// Return Value:
//       True indicates success; otherwise false.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnLButtonUp(FPDF_FORMHANDLE hHandle,
                                                     FPDF_PAGE page,
                                                     int modifier,
                                                     double page_x,
                                                     double page_y);

// Function: FORM_OnRButtonUp
//       Same as above, execpt for the right mouse button.
// Comments:
//       At the present time, has no effect except in XFA builds, but is
//       included for the sake of symmetry.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnRButtonUp(FPDF_FORMHANDLE hHandle,
                                                     FPDF_PAGE page,
                                                     int modifier,
                                                     double page_x,
                                                     double page_y);

// Function: FORM_OnLButtonDoubleClick
//       Call this member function when the user double clicks the
//       left mouse button.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       modifier    -   Indicates whether various virtual keys are down.
//       page_x      -   Specifies the x-coordinate of the cursor in PDF user
//                       space.
//       page_y      -   Specifies the y-coordinate of the cursor in PDF user
//                       space.
// Return Value:
//       True indicates success; otherwise false.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FORM_OnLButtonDoubleClick(FPDF_FORMHANDLE hHandle,
                          FPDF_PAGE page,
                          int modifier,
                          double page_x,
                          double page_y);

// Function: FORM_OnKeyDown
//       Call this member function when a nonsystem key is pressed.
// Parameters:
//       hHandle     -   Handle to the form fill module, aseturned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       nKeyCode    -   The virtual-key code of the given key (see
//                       fpdf_fwlevent.h for virtual key codes).
//       modifier    -   Mask of key flags (see fpdf_fwlevent.h for key
//                       flag values).
// Return Value:
//       True indicates success; otherwise false.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnKeyDown(FPDF_FORMHANDLE hHandle,
                                                   FPDF_PAGE page,
                                                   int nKeyCode,
                                                   int modifier);

// Function: FORM_OnKeyUp
//       Call this member function when a nonsystem key is released.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       nKeyCode    -   The virtual-key code of the given key (see
//                       fpdf_fwlevent.h for virtual key codes).
//       modifier    -   Mask of key flags (see fpdf_fwlevent.h for key
//                       flag values).
// Return Value:
//       True indicates success; otherwise false.
// Comments:
//       Currently unimplemented and always returns false. PDFium reserves this
//       API and may implement it in the future on an as-needed basis.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnKeyUp(FPDF_FORMHANDLE hHandle,
                                                 FPDF_PAGE page,
                                                 int nKeyCode,
                                                 int modifier);

// Function: FORM_OnChar
//       Call this member function when a keystroke translates to a
//       nonsystem character.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       nChar       -   The character code value itself.
//       modifier    -   Mask of key flags (see fpdf_fwlevent.h for key
//                       flag values).
// Return Value:
//       True indicates success; otherwise false.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_OnChar(FPDF_FORMHANDLE hHandle,
                                                FPDF_PAGE page,
                                                int nChar,
                                                int modifier);

// Experimental API
// Function: FORM_GetFocusedText
//       Call this function to obtain the text within the current focused
//       field, if any.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       buffer      -   Buffer for holding the form text, encoded in
//                       UTF-16LE. If NULL, |buffer| is not modified.
//       buflen      -   Length of |buffer| in bytes. If |buflen| is less
//                       than the length of the form text string, |buffer| is
//                       not modified.
// Return Value:
//       Length in bytes for the text in the focused field.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FORM_GetFocusedText(FPDF_FORMHANDLE hHandle,
                    FPDF_PAGE page,
                    void* buffer,
                    unsigned long buflen);

// Function: FORM_GetSelectedText
//       Call this function to obtain selected text within a form text
//       field or form combobox text field.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
//       buffer      -   Buffer for holding the selected text, encoded in
//                       UTF-16LE. If NULL, |buffer| is not modified.
//       buflen      -   Length of |buffer| in bytes. If |buflen| is less
//                       than the length of the selected text string,
//                       |buffer| is not modified.
// Return Value:
//       Length in bytes of selected text in form text field or form combobox
//       text field.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FORM_GetSelectedText(FPDF_FORMHANDLE hHandle,
                     FPDF_PAGE page,
                     void* buffer,
                     unsigned long buflen);

// Experimental API
// Function: FORM_ReplaceAndKeepSelection
//       Call this function to replace the selected text in a form
//       text field or user-editable form combobox text field with another
//       text string (which can be empty or non-empty). If there is no
//       selected text, this function will append the replacement text after
//       the current caret position. After the insertion, the inserted text
//       will be selected.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as Returned by FPDF_LoadPage().
//       wsText      -   The text to be inserted, in UTF-16LE format.
// Return Value:
//       None.
FPDF_EXPORT void FPDF_CALLCONV
FORM_ReplaceAndKeepSelection(FPDF_FORMHANDLE hHandle,
                             FPDF_PAGE page,
                             FPDF_WIDESTRING wsText);

// Function: FORM_ReplaceSelection
//       Call this function to replace the selected text in a form
//       text field or user-editable form combobox text field with another
//       text string (which can be empty or non-empty). If there is no
//       selected text, this function will append the replacement text after
//       the current caret position. After the insertion, the selection range
//       will be set to empty.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as Returned by FPDF_LoadPage().
//       wsText      -   The text to be inserted, in UTF-16LE format.
// Return Value:
//       None.
FPDF_EXPORT void FPDF_CALLCONV FORM_ReplaceSelection(FPDF_FORMHANDLE hHandle,
                                                     FPDF_PAGE page,
                                                     FPDF_WIDESTRING wsText);

// Experimental API
// Function: FORM_SelectAllText
//       Call this function to select all the text within the currently focused
//       form text field or form combobox text field.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
// Return Value:
//       Whether the operation succeeded or not.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FORM_SelectAllText(FPDF_FORMHANDLE hHandle, FPDF_PAGE page);

// Function: FORM_CanUndo
//       Find out if it is possible for the current focused widget in a given
//       form to perform an undo operation.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
// Return Value:
//       True if it is possible to undo.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_CanUndo(FPDF_FORMHANDLE hHandle,
                                                 FPDF_PAGE page);

// Function: FORM_CanRedo
//       Find out if it is possible for the current focused widget in a given
//       form to perform a redo operation.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
// Return Value:
//       True if it is possible to redo.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_CanRedo(FPDF_FORMHANDLE hHandle,
                                                 FPDF_PAGE page);

// Function: FORM_Undo
//       Make the current focused widget perform an undo operation.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
// Return Value:
//       True if the undo operation succeeded.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_Undo(FPDF_FORMHANDLE hHandle,
                                              FPDF_PAGE page);

// Function: FORM_Redo
//       Make the current focused widget perform a redo operation.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page        -   Handle to the page, as returned by FPDF_LoadPage().
// Return Value:
//       True if the redo operation succeeded.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FORM_Redo(FPDF_FORMHANDLE hHandle,
                                              FPDF_PAGE page);

// Function: FORM_ForceToKillFocus.
//       Call this member function to force to kill the focus of the form
//       field which has focus. If it would kill the focus of a form field,
//       save the value of form field if was changed by theuser.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
// Return Value:
//       True indicates success; otherwise false.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FORM_ForceToKillFocus(FPDF_FORMHANDLE hHandle);

// Experimental API.
// Function: FORM_GetFocusedAnnot.
//       Call this member function to get the currently focused annotation.
// Parameters:
//       handle      -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       page_index  -   Buffer to hold the index number of the page which
//                       contains the focused annotation. 0 for the first page.
//                       Can't be NULL.
//       annot       -   Buffer to hold the focused annotation. Can't be NULL.
// Return Value:
//       On success, return true and write to the out parameters. Otherwise
//       return false and leave the out parameters unmodified.
// Comments:
//       Not currently supported for XFA forms - will report no focused
//       annotation.
//       Must call FPDFPage_CloseAnnot() when the annotation returned in |annot|
//       by this function is no longer needed.
//       This will return true and set |page_index| to -1 and |annot| to NULL,
//       if there is no focused annotation.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FORM_GetFocusedAnnot(FPDF_FORMHANDLE handle,
                     int* page_index,
                     FPDF_ANNOTATION* annot);

// Experimental API.
// Function: FORM_SetFocusedAnnot.
//       Call this member function to set the currently focused annotation.
// Parameters:
//       handle      -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       annot       -   Handle to an annotation.
// Return Value:
//       True indicates success; otherwise false.
// Comments:
//       |annot| can't be NULL. To kill focus, use FORM_ForceToKillFocus()
//       instead.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FORM_SetFocusedAnnot(FPDF_FORMHANDLE handle, FPDF_ANNOTATION annot);

// Form Field Types
// The names of the defines are stable, but the specific values associated with
// them are not, so do not hardcode their values.
#define FPDF_FORMFIELD_UNKNOWN 0      // Unknown.
#define FPDF_FORMFIELD_PUSHBUTTON 1   // push button type.
#define FPDF_FORMFIELD_CHECKBOX 2     // check box type.
#define FPDF_FORMFIELD_RADIOBUTTON 3  // radio button type.
#define FPDF_FORMFIELD_COMBOBOX 4     // combo box type.
#define FPDF_FORMFIELD_LISTBOX 5      // list box type.
#define FPDF_FORMFIELD_TEXTFIELD 6    // text field type.
#define FPDF_FORMFIELD_SIGNATURE 7    // text field type.
#ifdef PDF_ENABLE_XFA
#define FPDF_FORMFIELD_XFA 8              // Generic XFA type.
#define FPDF_FORMFIELD_XFA_CHECKBOX 9     // XFA check box type.
#define FPDF_FORMFIELD_XFA_COMBOBOX 10    // XFA combo box type.
#define FPDF_FORMFIELD_XFA_IMAGEFIELD 11  // XFA image field type.
#define FPDF_FORMFIELD_XFA_LISTBOX 12     // XFA list box type.
#define FPDF_FORMFIELD_XFA_PUSHBUTTON 13  // XFA push button type.
#define FPDF_FORMFIELD_XFA_SIGNATURE 14   // XFA signture field type.
#define FPDF_FORMFIELD_XFA_TEXTFIELD 15   // XFA text field type.
#endif                                    // PDF_ENABLE_XFA

#ifdef PDF_ENABLE_XFA
#define FPDF_FORMFIELD_COUNT 16
#else  // PDF_ENABLE_XFA
#define FPDF_FORMFIELD_COUNT 8
#endif  // PDF_ENABLE_XFA

#ifdef PDF_ENABLE_XFA
#define IS_XFA_FORMFIELD(type)                  \
  (((type) == FPDF_FORMFIELD_XFA) ||            \
   ((type) == FPDF_FORMFIELD_XFA_CHECKBOX) ||   \
   ((type) == FPDF_FORMFIELD_XFA_COMBOBOX) ||   \
   ((type) == FPDF_FORMFIELD_XFA_IMAGEFIELD) || \
   ((type) == FPDF_FORMFIELD_XFA_LISTBOX) ||    \
   ((type) == FPDF_FORMFIELD_XFA_PUSHBUTTON) || \
   ((type) == FPDF_FORMFIELD_XFA_SIGNATURE) ||  \
   ((type) == FPDF_FORMFIELD_XFA_TEXTFIELD))
#endif  // PDF_ENABLE_XFA

// Function: FPDFPage_HasFormFieldAtPoint
//     Get the form field type by point.
// Parameters:
//     hHandle     -   Handle to the form fill module. Returned by
//                     FPDFDOC_InitFormFillEnvironment().
//     page        -   Handle to the page. Returned by FPDF_LoadPage().
//     page_x      -   X position in PDF "user space".
//     page_y      -   Y position in PDF "user space".
// Return Value:
//     Return the type of the form field; -1 indicates no field.
//     See field types above.
FPDF_EXPORT int FPDF_CALLCONV
FPDFPage_HasFormFieldAtPoint(FPDF_FORMHANDLE hHandle,
                             FPDF_PAGE page,
                             double page_x,
                             double page_y);

// Function: FPDFPage_FormFieldZOrderAtPoint
//     Get the form field z-order by point.
// Parameters:
//     hHandle     -   Handle to the form fill module. Returned by
//                     FPDFDOC_InitFormFillEnvironment().
//     page        -   Handle to the page. Returned by FPDF_LoadPage().
//     page_x      -   X position in PDF "user space".
//     page_y      -   Y position in PDF "user space".
// Return Value:
//     Return the z-order of the form field; -1 indicates no field.
//     Higher numbers are closer to the front.
FPDF_EXPORT int FPDF_CALLCONV
FPDFPage_FormFieldZOrderAtPoint(FPDF_FORMHANDLE hHandle,
                                FPDF_PAGE page,
                                double page_x,
                                double page_y);

// Function: FPDF_SetFormFieldHighlightColor
//       Set the highlight color of the specified (or all) form fields
//       in the document.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       doc         -   Handle to the document, as returned by
//                       FPDF_LoadDocument().
//       fieldType   -   A 32-bit integer indicating the type of a form
//                       field (defined above).
//       color       -   The highlight color of the form field. Constructed by
//                       0xxxrrggbb.
// Return Value:
//       None.
// Comments:
//       When the parameter fieldType is set to FPDF_FORMFIELD_UNKNOWN, the
//       highlight color will be applied to all the form fields in the
//       document.
//       Please refresh the client window to show the highlight immediately
//       if necessary.
FPDF_EXPORT void FPDF_CALLCONV
FPDF_SetFormFieldHighlightColor(FPDF_FORMHANDLE hHandle,
                                int fieldType,
                                unsigned long color);

// Function: FPDF_SetFormFieldHighlightAlpha
//       Set the transparency of the form field highlight color in the
//       document.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
//       doc         -   Handle to the document, as returaned by
//                       FPDF_LoadDocument().
//       alpha       -   The transparency of the form field highlight color,
//                       between 0-255.
// Return Value:
//       None.
FPDF_EXPORT void FPDF_CALLCONV
FPDF_SetFormFieldHighlightAlpha(FPDF_FORMHANDLE hHandle, unsigned char alpha);

// Function: FPDF_RemoveFormFieldHighlight
//       Remove the form field highlight color in the document.
// Parameters:
//       hHandle     -   Handle to the form fill module, as returned by
//                       FPDFDOC_InitFormFillEnvironment().
// Return Value:
//       None.
// Comments:
//       Please refresh the client window to remove the highlight immediately
//       if necessary.
FPDF_EXPORT void FPDF_CALLCONV
FPDF_RemoveFormFieldHighlight(FPDF_FORMHANDLE hHandle);

// Function: FPDF_FFLDraw
//       Render FormFields and popup window on a page to a device independent
//       bitmap.
// Parameters:
//       hHandle      -   Handle to the form fill module, as returned by
//                        FPDFDOC_InitFormFillEnvironment().
//       bitmap       -   Handle to the device independent bitmap (as the
//                        output buffer). Bitmap handles can be created by
//                        FPDFBitmap_Create().
//       page         -   Handle to the page, as returned by FPDF_LoadPage().
//       start_x      -   Left pixel position of the display area in the
//                        device coordinates.
//       start_y      -   Top pixel position of the display area in the device
//                        coordinates.
//       size_x       -   Horizontal size (in pixels) for displaying the page.
//       size_y       -   Vertical size (in pixels) for displaying the page.
//       rotate       -   Page orientation: 0 (normal), 1 (rotated 90 degrees
//                        clockwise), 2 (rotated 180 degrees), 3 (rotated 90
//                        degrees counter-clockwise).
//       flags        -   0 for normal display, or combination of flags
//                        defined above.
// Return Value:
//       None.
// Comments:
//       This function is designed to render annotations that are
//       user-interactive, which are widget annotations (for FormFields) and
//       popup annotations.
//       With the FPDF_ANNOT flag, this function will render a popup annotation
//       when users mouse-hover on a non-widget annotation. Regardless of
//       FPDF_ANNOT flag, this function will always render widget annotations
//       for FormFields.
//       In order to implement the FormFill functions, implementation should
//       call this function after rendering functions, such as
//       FPDF_RenderPageBitmap() or FPDF_RenderPageBitmap_Start(), have
//       finished rendering the page contents.
FPDF_EXPORT void FPDF_CALLCONV FPDF_FFLDraw(FPDF_FORMHANDLE hHandle,
                                            FPDF_BITMAP bitmap,
                                            FPDF_PAGE page,
                                            int start_x,
                                            int start_y,
                                            int size_x,
                                            int size_y,
                                            int rotate,
                                            int flags);

#if defined(PDF_USE_SKIA)
FPDF_EXPORT void FPDF_CALLCONV FPDF_FFLDrawSkia(FPDF_FORMHANDLE hHandle,
                                                FPDF_SKIA_CANVAS canvas,
                                                FPDF_PAGE page,
                                                int start_x,
                                                int start_y,
                                                int size_x,
                                                int size_y,
                                                int rotate,
                                                int flags);
#endif

// Experimental API
// Function: FPDF_GetFormType
//           Returns the type of form contained in the PDF document.
// Parameters:
//           document - Handle to document.
// Return Value:
//           Integer value representing one of the FORMTYPE_ values.
// Comments:
//           If |document| is NULL, then the return value is FORMTYPE_NONE.
FPDF_EXPORT int FPDF_CALLCONV FPDF_GetFormType(FPDF_DOCUMENT document);

// Experimental API
// Function: FORM_SetIndexSelected
//           Selects/deselects the value at the given |index| of the focused
//           annotation.
// Parameters:
//           hHandle     -   Handle to the form fill module. Returned by
//                           FPDFDOC_InitFormFillEnvironment.
//           page        -   Handle to the page. Returned by FPDF_LoadPage
//           index       -   0-based index of value to be set as
//                           selected/unselected
//           selected    -   true to select, false to deselect
// Return Value:
//           TRUE if the operation succeeded.
//           FALSE if the operation failed or widget is not a supported type.
// Comments:
//           Intended for use with listbox/combobox widget types. Comboboxes
//           have at most a single value selected at a time which cannot be
//           deselected. Deselect on a combobox is a no-op that returns false.
//           Default implementation is a no-op that will return false for
//           other types.
//           Not currently supported for XFA forms - will return false.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FORM_SetIndexSelected(FPDF_FORMHANDLE hHandle,
                      FPDF_PAGE page,
                      int index,
                      FPDF_BOOL selected);

// Experimental API
// Function: FORM_IsIndexSelected
//           Returns whether or not the value at |index| of the focused
//           annotation is currently selected.
// Parameters:
//           hHandle     -   Handle to the form fill module. Returned by
//                           FPDFDOC_InitFormFillEnvironment.
//           page        -   Handle to the page. Returned by FPDF_LoadPage
//           index       -   0-based Index of value to check
// Return Value:
//           TRUE if value at |index| is currently selected.
//           FALSE if value at |index| is not selected or widget is not a
//           supported type.
// Comments:
//           Intended for use with listbox/combobox widget types. Default
//           implementation is a no-op that will return false for other types.
//           Not currently supported for XFA forms - will return false.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FORM_IsIndexSelected(FPDF_FORMHANDLE hHandle, FPDF_PAGE page, int index);

// Function: FPDF_LoadXFA
//          If the document consists of XFA fields, call this method to
//          attempt to load XFA fields.
// Parameters:
//          document     -   Handle to document from FPDF_LoadDocument().
// Return Value:
//          TRUE upon success, otherwise FALSE. If XFA support is not built
//          into PDFium, performs no action and always returns FALSE.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_LoadXFA(FPDF_DOCUMENT document);

#ifdef __cplusplus
}
#endif

#endif  // PUBLIC_FPDF_FORMFILL_H_
