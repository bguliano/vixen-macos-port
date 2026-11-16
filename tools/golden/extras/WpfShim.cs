// UI/property-system hosts only. All point/matrix arithmetic and the original
// RotateTransform class come from the pinned MIT WPF source in helpers/reference.
// prepare.py extracts the unchanged call path; WPF native drawing is never used.
namespace System.Windows {
    public struct Point {
        internal double _x, _y;
        public double X { get => _x; set => _x = value; }
        public double Y { get => _y; set => _y = value; }
    }
}
namespace System.Windows.Media {
    public partial struct Matrix {
        internal double _m11, _m12, _m21, _m22, _offsetX, _offsetY;
        internal MS.Internal.MatrixTypes _type;
        internal int _padding;
    }
    public abstract partial class GeneralTransform {
        public abstract bool TryTransform(System.Windows.Point point, out System.Windows.Point result);
    }
    public abstract partial class Transform : GeneralTransform {
        public abstract Matrix Value { get; }
        internal abstract bool IsIdentity { get; }
        protected void ReadPreamble() {}
        protected bool CanFreeze => true;
    }
    public sealed partial class RotateTransform {
        public double Angle { get; set; }
        public double CenterX { get; set; }
        public double CenterY { get; set; }
    }
    internal static class SR {
        public const string GeneralTransform_TransformFailed = "Transform failed.";
        public static string Format(string value, object argument) => value;
    }
}
