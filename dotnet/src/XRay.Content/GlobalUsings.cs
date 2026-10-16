// The domain enum XRay.Content.Types.UriKind (Hyperlink/Anchor/Email/…) collides with
// System.UriKind under ImplicitUsings. No library code uses System.UriKind, so alias
// the bare name to the domain type globally; qualify System.UriKind explicitly if ever needed.
global using UriKind = XRay.Content.Types.UriKind;
global using Cancel = XRay.Content.Core.Cancel;
