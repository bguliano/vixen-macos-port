# Original NCalc curve-generator oracle

This harness extracts `btnFunctionCurve_Click` and `CalculateFunction` verbatim
from the unchanged upstream `CurveEditor.cs`. It compiles those methods against
**NCalc2 2.1.0**, the version pinned in upstream `Directory.Packages.props`, and
its Antlr4.Runtime 4.6.4 dependency. Only the modal dialog, message box and graph
hosts are replaced. Parsing, numeric type promotion, expression evaluation,
clamping, interval iteration and exception handling all run in the original code.

The public NuGet archives and extracted DLLs are held privately in
`target/ncalc-reference`; the hashes of the source, version pin, archives and DLLs
are in `source-manifest.sha256`. The reference uses the existing private .NET 10
farm and offline NuGet configuration, with all generated state under
`target/golden`. It does not alter upstream, install software or contact a service.
The package ID is `NCalc2`; the archive name `ncalc2.2.1.0.nupkg` contains version
`2.1.0`, not version `2.2.1.0`.

```sh
tools/golden/ncalc/build.sh run
source env.sh
NCALC_GOLDEN="$PWD/target/golden/ncalc/data/ncalc.json" \
  cargo test -p vixen-session --lib original_ncalc_curve_contracts -- --nocapture
```

Normal Rust tests use the checked-in `tests/golden/ncalc.json` without .NET or the
reference DLLs. **4,282 cases, 8,650 generated points** pass, including 2,129 error
cases. The comparison checks point counts/order, coordinates (absolute tolerance
1e-10 for finite y; NaN matched explicitly), success/failure and error-box title.
The original error text and remembered expression are retained in the artifact
for inspection. The test does **not** compare diagnostic wording byte for byte.

Coverage includes every documented function; function, parameter and keyword
case; integer/long/double/decimal/bool/string combinations; boolean and bitwise
operators; conditional and lazy evaluation; NaN/infinity; integer overflow and
shift behavior; decimal limits and rounding; invariant date literals; escaped
strings; bad syntax/arity/type and interval inputs; and failure after some points
have already been added. Original interval steps 25, 30 and 100 cover both exact
endpoints and the extra final point. The dialog regression additionally proves
that a later failure retains the earlier generated points and remembers the
expression, and that a new failed expression clears the old graph first.

The oracle proved and corrected parameter-case handling, typed numeric/boolean
operators, integer shifts and long division, Decimal-returning Abs, Min/Max type
promotion, NaN comparisons, Log/IEEERemainder/Round boundaries, string conversion
and escaping, and lost partial graph points. Integer values above 2^53 and the
96-bit Decimal maximum exposed why an f64-backed Decimal approximation was not
sufficient. The port therefore pins **rust_decimal 1.43.0**, with default features
disabled, for checked 96-bit arithmetic and scale. The crate is MIT, requires Rust
1.67.1 (edition 2021), and reuses the workspace's existing arrayvec and num-traits;
its notice is bundled in `app/Resources/Licenses/rust_decimal.txt`. Decimal text
and double conversions handle the original midpoint-even and 15-significant-digit
rules explicitly before arithmetic. Primary package documentation:
https://docs.rs/rust_decimal/1.43.0/rust_decimal/.

Limits: this is a broad contract matrix, not an exhaustive language proof. It runs
with invariant culture on .NET 10; other locale number/date formats, original
Windows framework diagnostic text and all possible grammar inputs are not claimed.
Zero/negative intervals are intentionally rejected by the port instead of entering
the original nonterminating loop. Cancel is a UI no-op and is not an expression
case. This harness does not exercise the native Function Generator sheet; existing
native UI checks and the Rust dialog tests cover that separate path.
