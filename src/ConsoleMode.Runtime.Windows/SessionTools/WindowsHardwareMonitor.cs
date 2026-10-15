// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/HardwareMonitor.cs, Native/PerfCounters.cs, Native/GpuKmt.cs
// e Native/SystemMemory.cs
using System.Runtime.InteropServices;
using ConsoleMode.Runtime.SessionTools;
using static ConsoleMode.Runtime.Windows.SessionTools.SessionToolsNative;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// CPU, GPU e memória pelos contadores do próprio Windows (PDH, a fonte do Gerenciador de Tarefas) e pelo driver de
/// vídeo (D3DKMT): nada a instalar e sem administrador. Uso precisa de duas amostras, então a primeira vem sem ele.
/// Seguro entre threads: uma amostra por vez.
/// </summary>
public sealed class WindowsHardwareMonitor : IHardwareMonitor, IDisposable
{
    private readonly object _gate = new();
    private Counters? _counters;
    private bool _disposed;

    /// <summary>Os contadores só abrem na primeira leitura: criar o serviço não custa nada na abertura do Tako.</summary>
    public HardwareSample Sample()
    {
        lock (_gate)
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            var c = _counters ??= new Counters();
            c.Pdh.Collect();
            double? frequency = PerfCounters.Read(c.CpuFrequency);
            double? performance = PerfCounters.Read(c.CpuPerformance);
            var dedicated = c.Pdh.ReadAll(c.GpuDedicated);
            var gpu = GpuCounters.Busiest(c.Pdh.ReadAll(c.GpuEngines), dedicated);
            var kmt = gpu is { } a ? GpuKmt.Read(a.LuidLow, a.LuidHigh, a.Node) : default;
            var (ramUsed, ramTotal) = ReadMemory();

            return new HardwareSample
            {
                CpuUsage = PerfCounters.Read(c.CpuUsage) is { } cpu ? Math.Clamp(cpu, 0, 100) : null,
                CpuClockMhz = frequency is > 0 && performance is > 0 ? frequency * performance / 100 : null,
                GpuUsage = gpu?.Usage,
                GpuClockMhz = kmt.ClockMhz,
                GpuTempC = kmt.TemperatureC,
                VramUsedMb = gpu is { } g ? GpuCounters.DedicatedMb(dedicated, g.Key) : null,
                VramTotalMb = kmt.DedicatedMb,
                RamUsedMb = ramUsed,
                RamTotalMb = ramTotal,
            };
        }
    }

    public void Dispose()
    {
        lock (_gate)
        {
            _disposed = true;
            _counters?.Pdh.Dispose();
            _counters = null;
        }
    }

    /// <summary>RAM em uso e instalada, em MB; nulos se o Windows não responder.</summary>
    private static unsafe (double? UsedMb, double? TotalMb) ReadMemory()
    {
        var status = new MemoryStatusEx { Length = (uint)sizeof(MemoryStatusEx) };
        if (!GlobalMemoryStatusEx(&status) || status.TotalPhys == 0)
        {
            return (null, null);
        }

        const double Mb = 1024 * 1024;
        return ((status.TotalPhys - status.AvailPhys) / Mb, status.TotalPhys / Mb);
    }

    private sealed class Counters
    {
        public Counters()
        {
            // "% Processor Utility" é o que o Gerenciador de Tarefas mostra (conta o turbo); sistemas antigos não têm.
            CpuUsage = Pdh.Add(@"\Processor Information(_Total)\% Processor Utility");
            if (CpuUsage == 0)
            {
                CpuUsage = Pdh.Add(@"\Processor Information(_Total)\% Processor Time");
            }

            // Clock efetivo = frequência base × % de desempenho, a "Velocidade" do Gerenciador de Tarefas.
            CpuPerformance = Pdh.Add(@"\Processor Information(_Total)\% Processor Performance");
            CpuFrequency = Pdh.Add(@"\Processor Information(_Total)\Processor Frequency");
            GpuEngines = Pdh.Add(@"\GPU Engine(*)\Utilization Percentage");
            GpuDedicated = Pdh.Add(@"\GPU Adapter Memory(*)\Dedicated Usage");
            Pdh.Collect();
        }

        public PerfCounters Pdh { get; } = new();

        public nint CpuUsage { get; }

        public nint CpuPerformance { get; }

        public nint CpuFrequency { get; }

        public nint GpuEngines { get; }

        public nint GpuDedicated { get; }
    }

    /// <summary>Contadores PDH com caminhos em inglês (PdhAddEnglishCounter): funcionam no Windows em qualquer idioma.</summary>
    private sealed unsafe class PerfCounters : IDisposable
    {
        private const uint PdhFmtDouble = 0x00000200;
        private const uint PdhFmtNoCap100 = 0x00008000;
        private const int PdhMoreData = unchecked((int)0x800007D2);

        private nint _query;

        public PerfCounters()
        {
            nint query;
            _query = PdhOpenQuery(null, 0, &query) == 0 ? query : 0;
        }

        /// <summary>0 quando o contador não existe nesta máquina (Windows antigo, sem driver de GPU…).</summary>
        public nint Add(string path)
        {
            nint counter;
            return _query != 0 && PdhAddEnglishCounter(_query, path, 0, &counter) == 0 ? counter : 0;
        }

        public void Collect()
        {
            if (_query != 0)
            {
                _ = PdhCollectQueryData(_query);
            }
        }

        public static double? Read(nint counter)
        {
            if (counter == 0)
            {
                return null;
            }

            uint type;
            FmtCounterValue value;
            return PdhGetFormattedCounterValue(counter, PdhFmtDouble | PdhFmtNoCap100, &type, &value) == 0 && value.Status == 0 ? value.Value : null;
        }

        /// <summary>Toda instância de um contador com curinga (…(*)\…) com seu valor.</summary>
        public List<(string Instance, double Value)> ReadAll(nint counter)
        {
            var result = new List<(string, double)>();
            if (counter == 0)
            {
                return result;
            }

            uint size = 0;
            uint count;
            if (PdhGetFormattedCounterArray(counter, PdhFmtDouble | PdhFmtNoCap100, &size, &count, null) != PdhMoreData || size == 0)
            {
                return result;
            }

            byte[] buffer = new byte[size];
            fixed (byte* p = buffer)
            {
                if (PdhGetFormattedCounterArray(counter, PdhFmtDouble | PdhFmtNoCap100, &size, &count, p) != 0)
                {
                    return result;
                }

                var items = (FmtCounterValueItem*)p;
                for (int i = 0; i < count; i++)
                {
                    if (items[i].Value.Status == 0 && Marshal.PtrToStringUni(items[i].Name) is { } name)
                    {
                        result.Add((name, items[i].Value.Value));
                    }
                }
            }

            return result;
        }

        public void Dispose()
        {
            if (_query != 0)
            {
                _ = PdhCloseQuery(_query);
                _query = 0;
            }
        }
    }

    /// <summary>
    /// Temperatura, clock e VRAM da GPU direto do driver (D3DKMT, WDDM 2.4+), a fonte da temperatura do Gerenciador de
    /// Tarefas. Drivers que não informam um valor deixam 0, lido aqui como nulo.
    /// </summary>
    private static unsafe class GpuKmt
    {
        private const int KmtqaiGetSegmentSize = 3;
        private const int KmtqaiNodePerfData = 61;
        private const int KmtqaiAdapterPerfData = 62;

        public readonly record struct Reading(double? TemperatureC, double? ClockMhz, double? DedicatedMb);

        public static Reading Read(uint luidLow, int luidHigh, uint node)
        {
            var open = new OpenAdapterFromLuid { LuidLow = luidLow, LuidHigh = luidHigh };
            try
            {
                if (D3DKMTOpenAdapterFromLuid(&open) != 0)
                {
                    return default;
                }
            }
            catch (EntryPointNotFoundException)
            {
                return default;
            }

            try
            {
                var adapter = default(AdapterPerfData);
                var engine = new NodePerfData { NodeOrdinal = node };
                var segments = default(SegmentSizeInfo);
                double? temperature = Query(open.Adapter, KmtqaiAdapterPerfData, &adapter) && adapter.Temperature > 0 ? adapter.Temperature / 10.0 : null;
                double? clock = Query(open.Adapter, KmtqaiNodePerfData, &engine) && engine.Frequency > 0 ? engine.Frequency / 1_000_000.0 : null;
                double? dedicated = Query(open.Adapter, KmtqaiGetSegmentSize, &segments) && segments.DedicatedVideoMemorySize > 0
                    ? segments.DedicatedVideoMemorySize / (1024.0 * 1024.0)
                    : null;
                return new Reading(temperature, clock, dedicated);
            }
            finally
            {
                uint handle = open.Adapter;
                _ = D3DKMTCloseAdapter(&handle);
            }
        }

        private static bool Query<T>(uint adapter, int type, T* data)
            where T : unmanaged
        {
            var query = new QueryAdapterInfo { Adapter = adapter, Type = type, Data = (nint)data, DataSize = (uint)sizeof(T) };
            return D3DKMTQueryAdapterInfo(&query) == 0;
        }
    }
}
