using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json;

// Only UI hosts are replaced. The original two handler/evaluator methods are
// copied verbatim at build time; all parsing/evaluation uses actual NCalc2.
public enum DialogResult { OK, Cancel }
public enum MessageBoxButtons { OK }
public static class SystemIcons { public static object Error => null; }
public class FunctionGenerator {
    public static string Input;
    public static bool Cancel;
    public string Function => Input;
    public FunctionGenerator(string previous) {}
    public DialogResult ShowDialog(object owner) => Cancel ? DialogResult.Cancel : DialogResult.OK;
}
public class MessageBoxForm {
    readonly string message, title;
    public MessageBoxForm(string message, string title, MessageBoxButtons buttons, object icon) { this.message=message; this.title=title; }
    public void ShowDialog(object owner) { var editor=(OriginalCurveEditor)owner; editor.Error=message; editor.Title=title; }
}
public class Points : List<(double x,double y)> { public void AddPoint(double x, double y) => Add((x,y)); }
public class Pane { public List<Points> CurveList = new(){new()}; }
public class Graph { public Pane GraphPane = new(); public void Invalidate() {} }
public class Field { public string Text; }
public partial class OriginalCurveEditor {
    string _holdFunction = "previous";
    Graph zedGraphControl = new();
    Field textBoxThreshold = new();
    public string Error, Title;
    public object Run(string expression,string step,bool cancel) {
        FunctionGenerator.Input=expression; FunctionGenerator.Cancel=cancel; textBoxThreshold.Text=step;
        zedGraphControl.GraphPane.CurveList[0].AddPoint(12,34);
        btnFunctionCurve_Click(null,EventArgs.Empty);
        return new { expression, step, cancel, points=zedGraphControl.GraphPane.CurveList[0].Select(p=>new object[]{p.x, double.IsFinite(p.y)?(object)p.y:p.y.ToString(CultureInfo.InvariantCulture)}), error=Error,title=Title,held=_holdFunction };
    }
}
public static class Program {
    public static void Main(string[] args) {
        CultureInfo.CurrentCulture=CultureInfo.InvariantCulture;
        CultureInfo.CurrentUICulture=CultureInfo.InvariantCulture;
        if(args.Length>0 && args[0]=="eval") { foreach(var e in args.Skip(1)) Console.WriteLine(JsonSerializer.Serialize(new OriginalCurveEditor().Run(e,"25",false))); return; }
        var expressions=new List<string>{
            "", "x", "X", "Pi", "PI", "pi", "[x]", "[X]", "[Pi]", "[pi]", "x/100*100", "x*x*x*x*x", "x%7", "-x+100",
            "Pow(x/100, 2.2)*80", "pow(X/100, 2)*100 + SIN(PI)*0", "Sin(2 * Pi * (x / 100)) * ((25-7)/2) + ((25-7)/2) + 7",
            "if(x>50,80,20)", "x>50?80:20", "in(x,0,1) and not false ? 10 : 90", "Max(x,10)-Min(x,10)",
            "if(x<50,x,unknown(0))", "if(x<50,x,unknown)", "if(x<50,x,'bad')", "if(x<50,x,1%0)",
            "unknown", "unknown(0)", "[unknown]", "x+", "(x", "x ?", "Sin()", "Sin(1,2)", "if(1,2)", "in(1)",
            "true", "false", "True", "FALSE", "null", "'42'", "'bad'", "'1'+'2'", "'1'+2", "1+'2'", "'a'='A'",
            "true+1", "true=1", "true<1", "true and false", "1 and 2", "not 0", "!true", "1|2", "1&3", "3^1", "1<<3", "8>>2", "~0", "~65535", "65536|1", "-1&3", "1<<-1", "1<<32",
            "1/0", "0/0", "1%0", "Sqrt(-1)", "Log(-1)", "Log(0)", "Log(100,10)", "Exp(1000)", "Pow(0,-1)", "Sign(Sqrt(-1))",
            "1e2", "1E-2", ".5", "1.", "2147483647+1", "2147483648", "2147483648+1", "9223372036854775807", "9223372036854775807+1", "-2147483648/-1", "Abs(-2147483648)",
            "Abs(x-50)/100", "Abs(x-50)+0.5", "Abs(x-50)*0.5", "Abs(-5)", "Abs(-5.2)", "Round(2.5)", "Round(3.5)", "Round(2.345,2)", "Round(2.345,-1)", "Round(2.345,16)",
            "if(true,10,unknown)", "if(false,unknown,10)", "false and unknown", "true or unknown", "in(1,1,unknown)", "in(2,1,unknown)",
            "#2020-01-01#", "#2020-01-01# = #2020-01-01#", "'abc' > 'abd'", "'10' < 2", "2 < '10'", "[Pi] * 10", "X + 1", "PI*10"
        };
        foreach(var function in new[]{"Abs","Acos","Asin","Atan","Ceiling","Cos","Exp","Floor","Log10","Sign","Sin","Sqrt","Tan","Truncate"})
            foreach(var value in new[]{"-1","0","0.5","1","50","x/100"}) expressions.Add($"{function}({value})");
        foreach(var function in new[]{"IEEERemainder","Log","Max","Min","Pow","Round"})
            foreach(var values in new[]{"5,2","-5,2","x,3","0,0","1,0","1.5,2"}) expressions.Add($"{function}({values})");
        foreach(var left in new[]{"1","2147483648","0.5","Abs(-2)","true","'2'","Sqrt(-1)"})
            foreach(var right in new[]{"1","2147483648","0.5","Abs(-2)","true","'2'","Sqrt(-1)"})
                foreach(var op in new[]{"+","-","*","/","%","=","!=","<",">","&","|","^","<<",">>","and","or"}) expressions.Add($"{left} {op} {right}");
        expressions.AddRange(new[]{"1 AND 2","true OR false","NOT false","if(1,20,30)","1?20:30","in(true,1)","in('2',2)","-true","~true","~0.5","Abs(1e30)","Abs(Sqrt(-1))","Abs(1.23456789012345678)","Abs(-2)-0.5","0.5-Abs(-2)","Abs(0.1)*3","3*Abs(0.1)","(Abs(0.1)*3)-0.3","x + 2 * 3", "'12\\'3'", "1e", "1.", "Sin()"});
        foreach(var function in new[]{"Min","Max","Log","IEEERemainder","Round"})
            foreach(var left in new[]{"-1","0","1","2","Abs(2)","true","'2'","Exp(1000)","Sqrt(-1)"})
                foreach(var right in new[]{"0","1","2","Abs(2)","true","'2'","Exp(1000)","Sqrt(-1)"}) expressions.Add($"{function}({left},{right})");
        expressions.AddRange(new[]{"(Abs(-2)+1)*0.5","(Abs(-2)+Abs(-3))*0.5","Abs(9223372036854775807)-9223372036854775806", "Abs(9007199254740993)-9007199254740992", "Abs(0.1)*3-Abs(0.3)", "Round(10000000000000001.1,1)-10000000000000000", "1/2147483649", "2147483648/2147483649", "Max(Abs(2),0.5)*0.5", "Min(Abs(2),3)*0.5", "#1/2/2020# = #2020-01-02#", "#2020-02-31#", "#not a date#"});
        expressions.AddRange(new[]{"Abs(1e-30)","Abs(5e-29)","Abs(6e-29)","Abs(9e-29)","Abs(1.234567890123456e-28)","Abs(2.5e-28)*Abs(1e28)","Abs(5e-29)*Abs(1e28)","Abs(6e-29)*Abs(1e28)","Abs(79228162514264337593543950335)","Abs('79228162514264337593543950335')-Abs('79228162514264337593543950334')","Abs(0.00000000000000000000000000005)*100","'\\u0032'","'\\t12\\n'","if(' true ',2,3)","'1,000'","Abs('0.00000000000000000000000000005')*Abs(1e28)","Abs('2-')"});
        expressions.AddRange(new[]{"Abs(',2')","Abs('2,')","Abs('2,,3')","Abs('2.0,1')","Abs('2 -')","Abs('- 2')","Abs('2 +')","Abs('2.00000000000000000000000000005')", "',2'", "'2,'", "'2,,3'", "'2.0,1'"});
        var rows=new List<object>();
        foreach(var expression in expressions.Distinct()) foreach(var step in new[]{"25","30","100"}) rows.Add(new OriginalCurveEditor().Run(expression,step,false));
        foreach(var step in new[]{"","word","32768"," 5 ","1.5","+25","1,000"}) rows.Add(new OriginalCurveEditor().Run("x",step,false));
        var destination=args.Length>0?args[0]:"target/golden/ncalc/data/ncalc.json";
        Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(destination)));
        using(var stream=File.Create(destination)) JsonSerializer.Serialize(stream,new{cases=rows});
        Console.WriteLine($"Wrote {rows.Count} cases to {destination}");
    }
}
