// What the upstream Morph sources reference that does not exist on macOS. The WPF arithmetic is copied from
// tools/golden/extras/WpfShim.cs. The Windows Forms parts are only the message box in the PolygonType setter.

namespace System.Windows
{
	public struct Point { public double X; public double Y; public Point(double x, double y) { X = x; Y = y; } }
}

namespace System.Windows.Media
{
	public sealed class RotateTransform(double angle, double centerX, double centerY)
	{
		public System.Windows.Point Transform(System.Windows.Point point)
		{
			var a = angle % 360.0;
			var radians = a * (Math.PI / 180.0);
			var sin = Math.Sin(radians);
			var cos = Math.Cos(radians);
			var dx = (centerX * (1.0 - cos)) + (centerY * sin);
			var dy = (centerY * (1.0 - cos)) - (centerX * sin);
			var (m11, m12, m21, m22) = (cos, sin, -sin, cos);
			var x = point.X;
			var y = point.Y;
			var xadd = y * m21 + dx;
			var yadd = x * m12 + dy;
			x *= m11;
			x += xadd;
			y *= m22;
			y += yadd;
			return new System.Windows.Point { X = x, Y = y };
		}
	}
}

namespace System.Windows.Forms
{
	public enum MessageBoxButtons { YesNo }
	public enum DialogResult { None, OK }
}

namespace Common.Controls
{
	public sealed class MessageBoxForm(string text, string caption, System.Windows.Forms.MessageBoxButtons buttons, System.Drawing.Icon icon)
	{
		public System.Windows.Forms.DialogResult ShowDialog() => System.Windows.Forms.DialogResult.None;
	}
}
