// Adaptado de DevTechProfile/CapFrameX (MIT, ver NOTICE.md): source/CapFrameX.PresentMonInterface/PresentMonCaptureService.cs
// (abrir o PresentMon com o stdout redirecionado, ler linha a linha e encerrar a sessão).
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using ConsoleMode.Runtime.SessionTools;
using Microsoft.Extensions.Logging;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Contador de FPS pelo PresentMon (ADR-0017). O PresentMon é filho do Tako dentro de um Job Object que o fecha junto
/// se o Tako cair; a sessão ETW com o nome do Tako (<see cref="PresentMonArguments.SessionName"/>) é parada no fim e,
/// se sobrar de uma queda, na abertura seguinte. Sessões de outros programas (CapFrameX, FrameView, Afterburner)
/// nunca são tocadas.
/// </summary>
public sealed partial class PresentMonFpsCounter : IFpsCounter, IDisposable
{
    private static readonly TimeSpan RefreshInterval = TimeSpan.FromSeconds(1);

    private readonly string _exePath;
    private readonly string _runningMarker;
    private readonly ILogger<PresentMonFpsCounter> _logger;
    private readonly TimeProvider _time;
    private readonly object _gate = new();
    private readonly FrameStatsWindow _window = new();
    private readonly Queue<string> _errorTail = new();
    private PresentMonReader _reader = new();
    private Process? _process;
    private nint _job;
    private ITimer? _refresh;
    private Func<IReadOnlyCollection<int>>? _gameProcesses;
    private FpsCounterStatus _status = FpsCounterStatus.Idle;

    /// <param name="exePath">PresentMon.exe distribuído com o Tako.</param>
    /// <param name="runningMarker">Arquivo que existe enquanto o PresentMon roda (para limpar depois de uma queda).</param>
    public PresentMonFpsCounter(string exePath, string runningMarker, ILogger<PresentMonFpsCounter> logger, TimeProvider? time = null)
    {
        _exePath = exePath;
        _runningMarker = runningMarker;
        _logger = logger;
        _time = time ?? TimeProvider.System;
    }

    public bool IsAvailable => File.Exists(_exePath);

    public FpsCounterStatus Status
    {
        get
        {
            lock (_gate)
            {
                return _status;
            }
        }
    }

    public FrameStats? Current
    {
        get
        {
            lock (_gate)
            {
                return _process is null ? null : _window.Snapshot(_time.GetElapsedTime(0));
            }
        }
    }

    public bool StartCapture(Func<IReadOnlyCollection<int>> gameProcesses)
    {
        ArgumentNullException.ThrowIfNull(gameProcesses);
        lock (_gate)
        {
            _gameProcesses = gameProcesses;
            if (_process is not null)
            {
                return true;
            }

            if (!IsAvailable)
            {
                _status = FpsCounterStatus.Unavailable;
                return false;
            }

            _reader = new PresentMonReader();
            _window.Clear();
            _errorTail.Clear();
            var info = new ProcessStartInfo(_exePath)
            {
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                WorkingDirectory = Path.GetDirectoryName(_exePath) ?? string.Empty,
            };
            foreach (string arg in PresentMonArguments.Build())
            {
                info.ArgumentList.Add(arg);
            }

            var process = new Process { StartInfo = info, EnableRaisingEvents = true };
            process.OutputDataReceived += (_, e) => OnLine(process, e.Data);
            process.ErrorDataReceived += (_, e) => OnError(process, e.Data);
            process.Exited += (_, _) => OnExited(process);
            try
            {
                if (!process.Start())
                {
                    process.Dispose();
                    _status = FpsCounterStatus.Failed;
                    return false;
                }
            }
            catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
            {
                process.Dispose();
                _status = FpsCounterStatus.Failed;
                LogStartFailed(_logger, ex);
                return false;
            }

            _process = process;
            _status = FpsCounterStatus.Starting;
            AttachToJob(process);
            process.BeginOutputReadLine();
            process.BeginErrorReadLine();
            MarkRunning(true);
            RefreshGameProcesses();
            _refresh = _time.CreateTimer(_ => OnRefresh(), null, RefreshInterval, RefreshInterval);
            LogStarted(_logger, process.Id);
            return true;
        }
    }

    public void StopCapture()
    {
        Process? process;
        lock (_gate)
        {
            process = _process;
            _process = null;
            _refresh?.Dispose();
            _refresh = null;
            _gameProcesses = null;
            _window.Clear();
            _status = IsAvailable ? FpsCounterStatus.Idle : FpsCounterStatus.Unavailable;
        }

        if (process is not null)
        {
            try
            {
                if (!process.HasExited)
                {
                    process.Kill();
                    process.WaitForExit(2000);
                }
            }
            catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
            {
                LogStopFailed(_logger, ex);
            }
            finally
            {
                process.Dispose();
            }
        }

        // Matar o PresentMon não fecha a sessão ETW dele: ela continuaria ocupando buffers do Windows.
        StopTakoTraceSession();
        MarkRunning(false);
    }

    public void RecoverAfterCrash()
    {
        if (!File.Exists(_runningMarker))
        {
            return;
        }

        LogRecovering(_logger);
        StopTakoTraceSession();
        MarkRunning(false);
    }

    public void Dispose()
    {
        StopCapture();
        if (_job != 0)
        {
            Native.CloseHandle(_job);
            _job = 0;
        }
    }

    private void OnLine(Process source, string? line)
    {
        if (line is null)
        {
            return;
        }

        lock (_gate)
        {
            if (!ReferenceEquals(source, _process))
            {
                return;
            }

            if (_reader.Columns is null)
            {
                _reader.TryRead(line, out _);
                if (_reader.Columns is not null)
                {
                    _status = FpsCounterStatus.Running; // o cabeçalho só sai com a sessão ETW aberta
                }
                else
                {
                    Remember(line);
                }

                return;
            }

            if (_reader.TryRead(line, out var sample))
            {
                _window.Add(sample, _time.GetElapsedTime(0));
            }
        }
    }

    private void OnError(Process source, string? line)
    {
        if (string.IsNullOrWhiteSpace(line))
        {
            return;
        }

        lock (_gate)
        {
            if (ReferenceEquals(source, _process))
            {
                Remember(line);
            }
        }
    }

    private void OnExited(Process source)
    {
        string tail;
        int code;
        lock (_gate)
        {
            if (!ReferenceEquals(source, _process))
            {
                return; // parado pelo Tako
            }

            tail = string.Join(" | ", _errorTail);
            _status = PresentMonDenied(tail) ? FpsCounterStatus.Denied : FpsCounterStatus.Failed;
            try
            {
                code = source.ExitCode;
            }
            catch (InvalidOperationException)
            {
                code = -1;
            }

            _process = null;
            _refresh?.Dispose();
            _refresh = null;
        }

        LogExited(_logger, code, tail);
        source.Dispose();
        MarkRunning(false);
    }

    /// <summary>O PresentMon avisa "requires either administrative privileges or ... Performance Log Users".</summary>
    private static bool PresentMonDenied(string output) =>
        output.Contains("Performance Log Users", StringComparison.OrdinalIgnoreCase)
        || output.Contains("administrative privileges", StringComparison.OrdinalIgnoreCase)
        || output.Contains("Access is denied", StringComparison.OrdinalIgnoreCase);

    private void Remember(string line)
    {
        _errorTail.Enqueue(line.Trim());
        while (_errorTail.Count > 6)
        {
            _errorTail.Dequeue();
        }
    }

    private void OnRefresh()
    {
        lock (_gate)
        {
            RefreshGameProcesses();
        }
    }

    /// <summary>Chamado com <see cref="_gate"/>: a lista de processos do jogo muda (launcher → jogo).</summary>
    private void RefreshGameProcesses()
    {
        if (_gameProcesses is not { } source)
        {
            return;
        }

        try
        {
            _window.SetGameProcesses(source());
        }
        catch (InvalidOperationException)
        {
            // A lista estava sendo trocada na thread da UI; a próxima leitura pega.
        }
    }

    private void AttachToJob(Process process)
    {
        try
        {
            if (_job == 0)
            {
                _job = Native.CreateJobKillOnClose();
            }

            if (_job != 0 && !Native.AssignProcessToJobObject(_job, process.Handle))
            {
                LogJobFailed(_logger, Marshal.GetLastPInvokeError());
            }
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
        {
            LogJobFailed(_logger, ex.HResult);
        }
    }

    private void StopTakoTraceSession()
    {
        int result = Native.StopTraceSession(PresentMonArguments.SessionName);
        if (result is not (0 or Native.ErrorWmiInstanceNotFound))
        {
            LogTraceStopFailed(_logger, result);
        }
    }

    private void MarkRunning(bool running)
    {
        try
        {
            if (running)
            {
                Directory.CreateDirectory(Path.GetDirectoryName(_runningMarker)!);
                File.WriteAllText(_runningMarker, PresentMonArguments.SessionName);
            }
            else if (File.Exists(_runningMarker))
            {
                File.Delete(_runningMarker);
            }
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogMarkerFailed(_logger, ex);
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "PresentMon aberto (pid {ProcessId})")]
    private static partial void LogStarted(ILogger logger, int processId);

    [LoggerMessage(Level = LogLevel.Warning, Message = "PresentMon não abriu")]
    private static partial void LogStartFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Warning, Message = "PresentMon fechou sozinho (código {ExitCode}): {Output}")]
    private static partial void LogExited(ILogger logger, int exitCode, string output);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Falha ao fechar o PresentMon")]
    private static partial void LogStopFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Warning, Message = "PresentMon fora do Job Object (erro {Error}); ele ainda fecha no fim da sessão")]
    private static partial void LogJobFailed(ILogger logger, int error);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Sessão ETW do Tako não pôde ser parada (erro {Error})")]
    private static partial void LogTraceStopFailed(ILogger logger, int error);

    [LoggerMessage(Level = LogLevel.Information, Message = "Fechando a sessão do PresentMon que uma execução anterior deixou aberta")]
    private static partial void LogRecovering(ILogger logger);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Marcador do PresentMon não pôde ser gravado")]
    private static partial void LogMarkerFailed(ILogger logger, Exception exception);

    private static unsafe partial class Native
    {
        public const int ErrorWmiInstanceNotFound = 4201;

        private const uint EventTraceControlStop = 1;
        private const int JobObjectExtendedLimitInformation = 9;
        private const uint JobObjectLimitKillOnJobClose = 0x2000;

        // EVENT_TRACE_PROPERTIES em 64 bits: 120 bytes; depois vêm os dois nomes (sessão e arquivo).
        private const int PropertiesSize = 120;
        private const int NameBytes = 1024 * sizeof(char);
        private const int LogFileNameOffsetField = 112;
        private const int LoggerNameOffsetField = 116;

        public static nint CreateJobKillOnClose()
        {
            nint job = CreateJobObject(0, null);
            if (job == 0)
            {
                return 0;
            }

            // JOBOBJECT_EXTENDED_LIMIT_INFORMATION (144 bytes em 64 bits); LimitFlags fica no byte 16.
            byte* info = stackalloc byte[144];
            new Span<byte>(info, 144).Clear();
            *(uint*)(info + 16) = JobObjectLimitKillOnJobClose;
            if (!SetInformationJobObject(job, JobObjectExtendedLimitInformation, info, 144))
            {
                CloseHandle(job);
                return 0;
            }

            return job;
        }

        /// <summary>ControlTrace(STOP) pelo nome; 0 = parada, 4201 = não existia.</summary>
        public static int StopTraceSession(string name)
        {
            if (!Environment.Is64BitProcess)
            {
                return ErrorWmiInstanceNotFound;
            }

            int size = PropertiesSize + (2 * NameBytes);
            nint buffer = Marshal.AllocHGlobal(size);
            try
            {
                new Span<byte>((void*)buffer, size).Clear();
                byte* p = (byte*)buffer;
                *(uint*)p = (uint)size; // Wnode.BufferSize
                *(uint*)(p + LoggerNameOffsetField) = PropertiesSize;
                *(uint*)(p + LogFileNameOffsetField) = PropertiesSize + NameBytes;
                return (int)ControlTrace(0, name, buffer, EventTraceControlStop);
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        [LibraryImport("kernel32.dll", EntryPoint = "CreateJobObjectW", StringMarshalling = StringMarshalling.Utf16, SetLastError = true)]
        private static partial nint CreateJobObject(nint attributes, string? name);

        [LibraryImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static partial bool SetInformationJobObject(nint job, int infoClass, void* info, uint length);

        [LibraryImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static partial bool AssignProcessToJobObject(nint job, nint process);

        [LibraryImport("kernel32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static partial bool CloseHandle(nint handle);

        [LibraryImport("advapi32.dll", EntryPoint = "ControlTraceW", StringMarshalling = StringMarshalling.Utf16)]
        private static partial uint ControlTrace(ulong handle, string name, nint properties, uint code);
    }
}
