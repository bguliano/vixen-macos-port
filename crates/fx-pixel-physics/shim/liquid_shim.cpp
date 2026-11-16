// The native half of the Liquid effect: upstream's C++/CLI LiquidFunWrapper
// (Vixen.Modules/Effect/Liquid/LiquidFunWrapper/LiquidFunWrapper.cpp) as plain C++ behind an extern "C" API.
// src/liquid/world.rs is the Rust side. Only the .NET types changed. The arithmetic is copied expression
// for expression, because mixed float and double expressions decide where particles end up.

#include <Box2D/Box2D.h>

#include <math.h>
#include <stddef.h>
#include <stdint.h>

#include <mutex>
#include <new>

namespace
{
	const int MaxColorIntensity = 255;

	// Math::PI
	const double Pi = 3.14159265358979323846;
	const float TWO_PI = (float)(2.0 * Pi);

	// WrapperParticleType, in declaration order. Rust passes the index.
	enum WrapperParticleType
	{
		Elastic,
		Powder,
		Tensile,
		Spring,
		Viscous,
		StaticPressure,
		Water,
		Reactive,
		Repulsive,
	};

	struct LiquidWorld
	{
		b2World* world;
		int width;
		int height;
		// The MSVC C runtime's rand() state. See rand01.
		uint32_t randState;
	};

	// rand() / RAND_MAX as the Windows build computes it. On Windows the state is per thread and RAND_MAX is
	// 0x7FFF. macOS has a process wide state and a much larger RAND_MAX. Effects render in parallel, so the
	// state lives in the world, which also makes a render repeatable from its seed.
	double rand01(LiquidWorld* w)
	{
		w->randState = w->randState * 214013u + 2531011u;
		const int value = (int)((w->randState >> 16) & 0x7FFF);
		return (double)value / (double)0x7FFF;
	}

	b2ParticleFlag ConvertParticleType(int particleType)
	{
		// The C# never passes anything outside the enum. A bad file gets the default particle.
		b2ParticleFlag particleFlag = b2_waterParticle;

		if (particleType == Elastic)
		{
			particleFlag = b2_elasticParticle;
		}
		else if (particleType == Powder)
		{
			particleFlag = b2_powderParticle;
		}
		else if (particleType == Tensile)
		{
			particleFlag = b2_tensileParticle;
		}
		else if (particleType == Spring)
		{
			particleFlag = b2_springParticle;
		}
		else if (particleType == Viscous)
		{
			particleFlag = b2_viscousParticle;
		}
		else if (particleType == StaticPressure)
		{
			particleFlag = b2_staticPressureParticle;
		}
		else if (particleType == Water)
		{
			particleFlag = b2_waterParticle;
		}
		else if (particleType == Reactive)
		{
			particleFlag = b2_reactiveParticle;
		}
		else if (particleType == Repulsive)
		{
			particleFlag = b2_repulsiveParticle;
		}

		return particleFlag;
	}

	void CreateParticles(
		LiquidWorld* w,
		b2ParticleSystem* ps,
		int x,
		int y,
		int direction,
		int velocity,
		int flow,
		int lifetime,
		int width,
		int height,
		const b2Color& color,
		int particleType,
		bool mixcolors,
		int sourceSize)
	{
		// Initialize the position of the particle
		float positionX = (float)x;
		float positionY = (float)y;

		float velocityX = (float)(velocity * 10.0 * cos(TWO_PI * direction / 360.0));
		float velocityY = (float)(velocity * 10.0 * sin(TWO_PI * direction / 360.0));

		float velocityVariation = (float)(rand01(w) * 0.1);
		velocityVariation -= velocityVariation / 2.0f;

		velocityX -= velocityX * velocityVariation;
		velocityY -= velocityY * velocityVariation;

		float particleLifetime = lifetime / 100.0f;

		// Create particles based on emitter flow
		for (int i = 0; i < flow; i++)
		{
			b2ParticleDef pd;
			pd.flags = ConvertParticleType(particleType);

			if (mixcolors)
			{
				pd.flags |= b2_colorMixingParticle;
			}

			pd.color.Set(color);

			if (sourceSize == 0)
			{
				// Randomly pick a position within the emitter's radius.
				const float32 angle = (float)(rand01(w) * 2.0 * b2_pi);

				// Distance from the center of the circle.
				const float32 distance = (float)rand01(w);
				b2Vec2 positionOnUnitCircle((float)sin(angle), (float)cos(angle));

				// Initial position.
				pd.position.Set(
					(float)(positionX + positionOnUnitCircle.x * distance * 0.5),
					(float)(positionY + positionOnUnitCircle.y * distance * 0.5));
			}
			else
			{
				// Distance from the center of the circle.
				const float32 distance = (float)(rand01(w) * ((float)sourceSize - (float)sourceSize / 2.0));

				float offsetX = (float)(distance * cos(TWO_PI * ((float)direction + 90.0) / 360.0));
				float offsetY = (float)(distance * sin(TWO_PI * ((float)direction + 90.0) / 360.0));

				// Initial position.
				pd.position.Set(positionX + (offsetX * (float)width / 200.0f), positionY + (offsetY * (float)height / 200.0f));
			}

			// Send particle flying
			pd.velocity.x = velocityX;
			pd.velocity.y = velocityY;

			// Give particle a lifetime
			if (lifetime > 0)
			{
				float randomlifeTime = (float)(particleLifetime + (particleLifetime * 0.2 * rand01(w)) - (particleLifetime * .01));
				pd.lifetime = randomlifeTime;
			}

			// If the lifetime is too small the Liquid Fun API seems to revert to a very large lifetime.
			if (pd.lifetime < 0.02)
			{
				pd.lifetime = 0.02f;
			}

			ps->CreateParticle(pd);
		}
	}

	void CreateParticleSystem(b2World* world, int lifetime, int size)
	{
		(void)lifetime; // Unused upstream too.

		b2ParticleSystemDef particleSystemDef;
		auto particleSystem = world->CreateParticleSystem(&particleSystemDef);
		particleSystem->SetRadius((float)size / 1000.0f);
		particleSystem->SetMaxParticleCount(100000);
		particleSystem->SetDestructionByAge(true);
	}

	void CreateBarrier(b2World* world, float x, float y, float width, float height)
	{
		b2BodyDef groundBodyDef;
		groundBodyDef.position.Set(x, y);

		// Create the rigid body in the world
		b2Body* groundBody = world->CreateBody(&groundBodyDef);

		// Set the bodies width and height
		b2PolygonShape groundBox;
		groundBox.SetAsBox(width / 2.0f, height / 2.0f);

		// Associate a fixture to the body
		groundBody->CreateFixture((b2Shape*)&groundBox, 0.0f);
	}

	// b2BlockAllocator fills a static lookup table the first time one is constructed, guarded by a plain bool.
	// Effects render in parallel, so build it once under call_once before any world can race on it.
	std::once_flag g_staticsReady;

	void PrepareStatics()
	{
		b2World world(b2Vec2(0.0f, 0.0f));
	}
}

extern "C"
{
	// EmitterWrapper. Its Brightness property is never read upstream, so it is left out.
	struct LfEmitter
	{
		int32_t x;
		int32_t y;
		uint8_t r;
		uint8_t g;
		uint8_t b;
		int32_t lifetime;
		int32_t velocity;
		int32_t source_size;
		int32_t flow;
		int32_t particle_type;
		int32_t angle;
	};

	// One element of what GetParticles returns.
	struct LfParticle
	{
		int32_t x;
		int32_t y;
		uint8_t r;
		uint8_t g;
		uint8_t b;
	};

	// LiquidFunWrapper::Initialize. Returns null if memory runs out.
	void* lf_world_create(
		int32_t width,
		int32_t height,
		bool bottom,
		bool top,
		bool left,
		bool right,
		int32_t particleSize,
		int32_t lifetime,
		uint32_t seed)
	{
		try
		{
			std::call_once(g_staticsReady, PrepareStatics);

			LiquidWorld* w = new LiquidWorld();
			w->width = width;
			w->height = height;
			w->randState = seed;

			b2Vec2 gravity;
			gravity.Set(0.0f, -10.0f);
			w->world = new b2World(gravity);

			CreateParticleSystem(w->world, lifetime, particleSize);

			if (bottom)
			{
				CreateBarrier(w->world, (float)width / 2.0f, -1.0f, (float)width, 0.001f);
			}
			if (top)
			{
				CreateBarrier(w->world, (float)width / 2.0f, height + 1.0f, (float)width, 0.001f);
			}
			if (left)
			{
				CreateBarrier(w->world, -1.0f, (float)height / 2.0f, 0.001f, (float)height);
			}
			if (right)
			{
				CreateBarrier(w->world, (float)width + 1.0f, (float)height / 2.0f, 0.001f, (float)height);
			}

			return w;
		}
		catch (...)
		{
			return nullptr;
		}
	}

	// LiquidFunWrapper::StepWorld.
	void lf_world_step(void* handle, float stepSeconds, bool mixColors, const LfEmitter* emitters, size_t emitterCount)
	{
		LiquidWorld* w = static_cast<LiquidWorld*>(handle);

		// Advance the world
		w->world->Step(
			stepSeconds, // Time
			6,           // Velocity Iterations
			2,           // Position Iterations
			3);          // Particle Iterations

		// Get the particle system(s)
		b2ParticleSystem* ps = w->world->GetParticleSystemList();

		// Loop over the emitters
		for (size_t e = 0; e < emitterCount; e++)
		{
			const LfEmitter& emitter = emitters[e];

			// Create the emitter color
			b2Color emitterColor(
				emitter.r / (float)MaxColorIntensity,
				emitter.g / (float)MaxColorIntensity,
				emitter.b / (float)MaxColorIntensity);

			// Create the emitter particles
			CreateParticles(
				w,
				ps,
				emitter.x,
				emitter.y,
				emitter.angle,
				emitter.velocity,
				emitter.flow,
				emitter.lifetime,
				w->width,
				w->height,
				emitterColor,
				emitter.particle_type,
				mixColors,
				emitter.source_size);
		}

		// Get the positions of the particles in the system
		const b2Vec2* positionBuffer = ps->GetPositionBuffer();

		// Get the number of particles in the system
		int particleCount = ps->GetParticleCount();

		// Loop over the particles
		for (int i = 0; i < particleCount; ++i)
		{
			// Get the position of the particle
			int x = (int)positionBuffer[i].x;
			int y = (int)positionBuffer[i].y;

			// If the particle is outside the viewable area then...
			if (y < -1 || x < -1 || x > w->width + 1)
			{
				// Destroy the particle
				ps->DestroyParticle(i);
			}
		}
	}

	// The length of the array LiquidFunWrapper::GetParticles allocates.
	size_t lf_world_particle_count(void* handle)
	{
		LiquidWorld* w = static_cast<LiquidWorld*>(handle);
		return (size_t)w->world->GetParticleSystemList()->GetParticleCount();
	}

	// LiquidFunWrapper::GetParticles, writing into a caller owned array so that a render reuses one buffer.
	// Returns how many it wrote.
	size_t lf_world_particles(void* handle, LfParticle* out, size_t capacity)
	{
		LiquidWorld* w = static_cast<LiquidWorld*>(handle);

		// Get the particle systems in the world
		b2ParticleSystem* particleSystems = w->world->GetParticleSystemList();

		// Get the particle position array
		const b2Vec2* positionBuffer = particleSystems->GetPositionBuffer();

		// Get the particle color array
		const b2ParticleColor* colorBuffer = particleSystems->GetColorBuffer();

		const size_t count = (size_t)particleSystems->GetParticleCount();
		const size_t n = count < capacity ? count : capacity;

		for (size_t i = 0; i < n; ++i)
		{
			// Get the position of the particle. rint is Math::Round(x, 0): ties go to the even value.
			out[i].x = (int32_t)rint((double)positionBuffer[i].x);
			out[i].y = (int32_t)rint((double)positionBuffer[i].y);

			// Get the color of the particle
			auto c = colorBuffer[i].GetColor();

			// Convert the particle
			out[i].r = (uint8_t)(int)(c.r * MaxColorIntensity);
			out[i].g = (uint8_t)(int)(c.g * MaxColorIntensity);
			out[i].b = (uint8_t)(int)(c.b * MaxColorIntensity);
		}

		return n;
	}

	// LiquidFunWrapper::FinalizeWorld. Upstream marks every particle destroyed first, which only flags them.
	void lf_world_destroy(void* handle)
	{
		LiquidWorld* w = static_cast<LiquidWorld*>(handle);
		delete w->world;
		delete w;
	}
}
