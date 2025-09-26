# MSG Test Files

This directory should contain real Microsoft Outlook MSG files for testing.

## Needed Test Files

### Simple MSG Files

- `simple/plain_text_only.msg` - Simple text-only MSG file
- `simple/html_email.msg` - MSG file with HTML content
- `simple/complex_headers.msg` - MSG file with multiple recipients

### MSG Files with Attachments

- `with_attachments/pdf_attachment.msg` - MSG file with PDF attachment
- `with_attachments/image_attachment.msg` - MSG file with image attachment
- `with_attachments/multiple_attachments.msg` - MSG file with multiple attachments

## How to Obtain MSG Files

1. **From Microsoft Outlook**: Drag and drop emails from Outlook to desktop
1. **From GitHub repositories**:
    - TeamMsgExtractor/msg-extractor (check example-msg-files directory)
    - tutao/oxmsg (check test directories)
1. **Create programmatically**: Use libraries like msg-extractor or oxmsg
1. **Download samples**: From file format documentation sites

## Testing Notes

- MSG files are binary CFB (Compound File Binary) format
- Our Rust implementation currently has limited MSG support
- Tests should verify both metadata extraction and attachment handling
- Need to test encoding handling for international characters
