// Upstream's descriptors with different type ids, because Vixen's loader drops both of two modules that share an id.

using Vixen.Module.Effect;
using Vixen.Sys;

namespace VixenModules.Effect.Bars
{
	public class GoldenBarsDescriptor : EffectModuleDescriptorBase
	{
		public override ParameterSignature Parameters => new();
		public override EffectGroups EffectGroup => EffectGroups.Pixel;
		public override string TypeName => EffectName;
		public override Guid TypeId => new("24decea6-78ab-4f42-acee-79afe4ffc016");
		public override Type ModuleClass => typeof(Bars);
		public override Type ModuleDataClass => typeof(BarsData);
		public override string Author => "Jeff Uchitjil";
		public override string Description => "Bars with upstream's WPF rotation arithmetic";
		public override string Version => "1.0";
		public override string EffectName => "Golden Bars";
	}
}

namespace VixenModules.Effect.Twinkle
{
	public class GoldenTwinkleDescriptor : EffectModuleDescriptorBase
	{
		public override ParameterSignature Parameters => new();
		public override EffectGroups EffectGroup => EffectGroups.Basic;
		public override string TypeName => EffectName;
		public override Guid TypeId => new("83bdd6f7-19c7-4598-b8e3-7ce28c44e7dc");
		public override Type ModuleClass => typeof(Twinkle);
		public override Type ModuleDataClass => typeof(TwinkleData);
		public override string Author => "Vixen Team";
		public override string Description => "Twinkle with a redirectable random source";
		public override string Version => "1.0";
		public override string EffectName => "Golden Twinkle";
	}
}

namespace VixenModules.Effect.Candle
{
	public class GoldenCandleDescriptor : EffectModuleDescriptorBase
	{
		public override ParameterSignature Parameters => new();
		public override EffectGroups EffectGroup => EffectGroups.Basic;
		public override string TypeName => EffectName;
		public override Guid TypeId => new("504582d2-43ac-472e-8885-ec4bcbf2b1f8");
		public override Type ModuleClass => typeof(CandleModule);
		public override Type ModuleDataClass => typeof(CandleData);
		public override string Author => "Vixen Team";
		public override string Description => "Candle with a redirectable random source";
		public override string Version => "1.0";
		public override string EffectName => "Golden Candle";
	}
}

namespace VixenModules.Effect.Pulse
{
	public class GoldenPulseDescriptor : EffectModuleDescriptorBase
	{
		public override ParameterSignature Parameters => new();
		public override EffectGroups EffectGroup => EffectGroups.Basic;
		public override string TypeName => EffectName;
		public override Guid TypeId => new("cbd76d3b-c924-40ff-bad6-d1437b3dbdc1");
		public override Type ModuleClass => typeof(Pulse);
		public override Type ModuleDataClass => typeof(PulseData);
		public override string Author => "Vixen Team";
		public override string Description => "Pulse as upstream has it";
		public override string Version => "1.0";
		public override string EffectName => "Golden Pulse";
	}
}

namespace VixenModules.Effect.SetLevel
{
	public class GoldenSetLevelDescriptor : EffectModuleDescriptorBase
	{
		public override ParameterSignature Parameters => new();
		public override EffectGroups EffectGroup => EffectGroups.Basic;
		public override string TypeName => EffectName;
		public override Guid TypeId => new("32cff8e0-5b10-4466-a093-0d232c55aac1");
		public override Type ModuleClass => typeof(SetLevel);
		public override Type ModuleDataClass => typeof(SetLevelData);
		public override string Author => "Vixen Team";
		public override string Description => "Set Level as upstream has it";
		public override string Version => "1.0";
		public override string EffectName => "Golden Set Level";
	}
}
