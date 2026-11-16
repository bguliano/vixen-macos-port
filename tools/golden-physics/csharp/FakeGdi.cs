// A stand-in for the parts of System.Drawing that Morph and FastPixel use, since GDI+ cannot load on macOS. The raster is
// the same logic as crates/fx-pixel-physics/src/morph/gdi.rs, so comparing frames checks everything AROUND the raster:
// which shapes get drawn, where, in what order and colour, with which masks, and when the effect throws. It says nothing
// about whether that raster matches real GDI+.
namespace System.Drawing
{
	public class Bitmap : IDisposable
	{
		public readonly Color[] Pixels;
		public readonly int W, H;
		public int Width => W;
		public int Height => H;
		public Bitmap(int w, int h) { if (w <= 0 || h <= 0) throw new ArgumentException("size"); W = w; H = h; Pixels = new Color[w * h]; }
		public void Dispose() { }
	}

	public class SolidBrush(Color c) : IDisposable { public Color Color = c; public void Dispose() { } }
	public class Pen(Color c) : IDisposable { public Color Color = c; public void Dispose() { } }

	public class Graphics : IDisposable
	{
		private readonly Bitmap _b;
		private double _tx, _ty, _angle;
		private Graphics(Bitmap b) { _b = b; }
		public static Graphics FromImage(Bitmap b) => new Graphics(b);
		public void Dispose() { }
		public void TranslateTransform(float x, float y) { _tx = x; _ty = y; }
		public void RotateTransform(float a) { _angle = a; }

		private void Plot(int x, int y, Color c)
		{
			if (x >= 0 && y >= 0 && x < _b.W && y < _b.H) _b.Pixels[y * _b.W + x] = c;
		}

		public void DrawLine(Pen p, Point a, Point b) => Line(a.X, a.Y, b.X, b.Y, p.Color);

		private void Line(int x0, int y0, int x1, int y1, Color c)
		{
			if (x0 == x1 && y0 == y1) return;
			int x = x0, y = y0;
			int dx = Math.Abs(x1 - x0), sx = x0 < x1 ? 1 : -1;
			int dy = Math.Abs(y1 - y0), sy = y0 < y1 ? 1 : -1;
			int err = (dx > dy ? dx : -dy) / 2;
			for (;;)
			{
				Plot(x, y, c);
				if (x == x1 && y == y1) break;
				int e2 = err;
				if (e2 > -dx) { err -= dy; x += sx; }
				if (e2 < dy) { err += dx; y += sy; }
			}
		}

		public void DrawPolygon(Pen p, Point[] pts)
		{
			if (pts.Length < 2) throw new ArgumentException("points");
			for (int i = 0; i < pts.Length; i++)
			{
				var a = pts[i]; var b = pts[(i + 1) % pts.Length];
				Line(a.X, a.Y, b.X, b.Y, p.Color);
			}
		}

		public void FillPolygon(SolidBrush b, Point[] pts)
		{
			if (pts.Length < 2) throw new ArgumentException("points");
			Fill(pts.Select(q => ((double)q.X, (double)q.Y)).ToArray(), b.Color);
		}

		private static double Snap(double v) => Math.Round(v * 16.0, MidpointRounding.AwayFromZero) / 16.0;

		private void Fill((double x, double y)[] raw, Color c)
		{
			if (raw.Length < 3) return;
			var pts = raw.Select(p => (x: Snap(p.x), y: Snap(p.y))).ToArray();
			double minY = pts.Min(p => p.y), maxY = pts.Max(p => p.y);
			int first = Math.Max((int)Math.Ceiling(minY), 0), last = Math.Min((int)Math.Ceiling(maxY), _b.H);
			var xs = new List<double>();
			for (int y = first; y < last; y++)
			{
				xs.Clear();
				for (int i = 0; i < pts.Length; i++)
				{
					var a = pts[i]; var b = pts[(i + 1) % pts.Length];
					var (t, u) = a.y <= b.y ? (a, b) : (b, a);
					if (t.y <= y && y < u.y) xs.Add(t.x + (y - t.y) * (u.x - t.x) / (u.y - t.y));
				}
				xs.Sort();
				for (int i = 0; i + 1 < xs.Count; i += 2)
				{
					int from = Math.Max((int)Math.Ceiling(xs[i]), 0), to = Math.Min((int)Math.Ceiling(xs[i + 1]), _b.W);
					for (int x = from; x < to; x++) Plot(x, y, c);
				}
			}
		}

		private (double x, double y)[] Vertices(Rectangle r, int steps)
		{
			double a = r.Width / 2.0, b = r.Height / 2.0;
			double ox = r.X + a, oy = r.Y + b;
			double rad = _angle * (Math.PI / 180.0);
			double sin = Math.Sin(rad), cos = Math.Cos(rad);
			var v = new (double, double)[steps];
			for (int i = 0; i < steps; i++)
			{
				double t = i * (Math.PI * 2) / steps;
				double lx = ox + a * Math.Cos(t), ly = oy + b * Math.Sin(t);
				v[i] = (_tx + lx * cos - ly * sin, _ty + lx * sin + ly * cos);
			}
			return v;
		}

		public void FillEllipse(SolidBrush b, Rectangle r)
		{
			if (r.Width <= 0 || r.Height <= 0) return;
			double radius = Math.Max(Math.Abs(r.Width), Math.Abs(r.Height)) / 2.0;
			int steps = (int)Math.Clamp(8.0 * Math.Ceiling(radius), 16.0, 4096.0);
			Fill(Vertices(r, steps), b.Color);
		}

		public void DrawEllipse(Pen p, Rectangle r)
		{
			double radius = Math.Max(Math.Abs(r.Width), Math.Abs(r.Height)) / 2.0;
			int steps = (int)Math.Clamp(4.0 * Math.Ceiling(Math.PI * radius / 4.0), 8.0, 4096.0);
			var v = Vertices(r, steps).Select(q => (x: (int)Math.Round(q.x, MidpointRounding.AwayFromZero), y: (int)Math.Round(q.y, MidpointRounding.AwayFromZero))).ToArray();
			for (int i = 0; i < v.Length; i++)
			{
				var a = v[i]; var b = v[(i + 1) % v.Length];
				Line(a.x, a.y, b.x, b.y, p.Color);
			}
		}
	}
}

namespace FastPixel
{
	public class FastPixel : IDisposable
	{
		private readonly System.Drawing.Bitmap _b;
		public FastPixel(System.Drawing.Bitmap b) { _b = b; }
		public FastPixel(int w, int h) { _b = new System.Drawing.Bitmap(w, h); }
		public void Lock() { }
		public void Unlock(bool setPixels) { }
		public void Dispose() { }
		public System.Drawing.Color GetPixel(int x, int y)
		{
			var c = _b.Pixels[y * _b.W + x];
			return System.Drawing.Color.FromArgb(c.A, c.R, c.G, c.B);
		}
		public void SetPixel(int x, int y, System.Drawing.Color c)
		{
			if (x >= 0 && x < _b.W && y >= 0 && y < _b.H) _b.Pixels[y * _b.W + x] = c;
		}
	}
}
