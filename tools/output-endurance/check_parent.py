"""Focused pure controls for the new bounded parent; no engine/native execution."""
import importlib.util,json,signal,tempfile
from pathlib import Path
from unittest.mock import patch,Mock
p=Path(__file__).with_name('run.py');s=importlib.util.spec_from_file_location('endurance',p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
checks=[]
child=Mock(pid=43210);child.poll.return_value=None
with patch.object(m.os,'killpg') as kill,patch.object(m.os,'_exit') as leave:
    m.deadline_abort({'child':child});kill.assert_called_once_with(43210,signal.SIGKILL);leave.assert_called_once_with(124)
checks.append('independent deadline kills only recorded owned group and exits failed')
with patch.object(m.os,'killpg') as kill,patch.object(m.os,'_exit') as leave:
    m.deadline_abort({});kill.assert_not_called();leave.assert_called_once_with(124)
checks.append('pre-spawn deadline performs no unowned kill')
m.termination_signal(signal.SIGTERM,None)
assert m.cancel_signal==signal.SIGTERM
try:m.check_cancel();raise AssertionError('signal ignored')
except SystemExit:checks.append('SIGTERM latches through spawn then routes into finally cleanup')
m.cancel_signal=None
with patch.object(m.os,'killpg') as kill:
    assert m.cleanup(child,m.time.monotonic()+10)==[];kill.assert_called_once_with(43210,signal.SIGTERM);child.wait.assert_called_once()
checks.append('cooperative cleanup waits on exact owned child')
child2=Mock(pid=54321);child2.poll.return_value=None;child2.wait.side_effect=[m.subprocess.TimeoutExpired('owned',4),0]
with patch.object(m.os,'killpg') as kill:
    assert m.cleanup(child2,m.time.monotonic()+10)==[]
    assert kill.call_args_list[0].args==(54321,signal.SIGTERM) and kill.call_args_list[1].args==(54321,signal.SIGKILL)
checks.append('cleanup escalation remains confined to owned group')
print(json.dumps(dict(passed=True,pureOnly=True,checks=checks),indent=2))
