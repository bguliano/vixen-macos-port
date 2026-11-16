#!/usr/bin/env python3
"""Read-only saved-state contract for the pending normal-app Vixen2 gestures."""
import hashlib
import io
import re
import xml.etree.ElementTree as ET
from pathlib import Path

NS='http://schemas.datacontract.org/2004/07/VixenModules.SequenceType.Vixen2x'
DRAW='http://schemas.datacontract.org/2004/07/System.Drawing'
ARRAYS='http://schemas.microsoft.com/2003/10/Serialization/Arrays'
XSI='http://www.w3.org/2001/XMLSchema-instance'
MODULE='92bbd2cb-b750-437f-8a88-49864d569ab4'
NIL='00000000-0000-0000-0000-000000000000'

def one(parent,tag):
    found=parent.findall(tag)
    assert len(found)==1,'Expected exactly one '+tag
    return found[0]

def text(parent,tag):return (one(parent,tag).text or '').strip()
def canonical(node):return [node.tag,[[k,v] for k,v in sorted(node.attrib.items())],(node.text or '').strip(),[canonical(c) for c in node]]

def typed_xml(data):
    """Resolve QName-valued xsi:type attributes while their namespace scope exists.

    ElementTree resolves element/attribute names but deliberately leaves attribute
    values lexical. Comparing c:int with d:int therefore requires their actual
    namespace bindings; an undefined prefix is invalid, not an ignorable alias.
    This tree is for read-only comparison, never serialization.
    """
    pending={};scopes=[];root=None
    for event,item in ET.iterparse(io.BytesIO(data),events=('start-ns','start','end')):
        if event=='start-ns':
            prefix,uri=item;pending[prefix or '']=uri
        elif event=='start':
            scope=dict(scopes[-1]) if scopes else {'xml':'http://www.w3.org/XML/1998/namespace'}
            scope.update(pending);pending.clear();scopes.append(scope)
            if root is None:root=item
            key='{'+XSI+'}type'
            if key in item.attrib:
                value=item.attrib[key];parts=value.split(':')
                assert len(parts) in [1,2] and all(re.fullmatch(r'[A-Za-z_][A-Za-z0-9_.-]*',p) for p in parts),'Malformed xsi:type QName'
                if len(parts)==2:
                    prefix,local=parts
                    assert prefix in scope and scope[prefix],'Undefined xsi:type namespace prefix: '+prefix
                    uri=scope[prefix]
                else:local=parts[0];uri=scope.get('','')
                item.set(key,'{'+uri+'}'+local if uri else local)
        else:
            scopes.pop()
    assert root is not None and not scopes,'Incomplete typed XML'
    return root

def color(parent,tag):
    node=one(parent,'{'+NS+'}'+tag)
    assert [c.tag for c in node]==['{'+DRAW+'}'+x for x in ['knownColor','name','state','value']]
    name=one(node,'{'+DRAW+'}name')
    assert name.attrib=={'{'+XSI+'}nil':'true'} and not (name.text or '').strip(),'Unexpected named-color payload'
    return {key:int(text(node,'{'+DRAW+'}'+key)) for key in ['knownColor','state','value']}

def value(argb):return dict(knownColor=0,state=2 if argb else 0,value=argb)

def saved_state(store,map_name):
    path=Path(store);data=path.read_bytes();assert len(data)<4_000_000
    root=typed_xml(data)
    modules=[m for m in root.iter('Module') if m.attrib.get('assemblyName')=='Module.SequenceType.Vixen2x']
    assert len(modules)==1,'Expected exactly one Vixen2 module'
    module=modules[0]
    assert module.attrib==dict(dataModelType='VixenModules.SequenceType.Vixen2x.Vixen2xSequenceStaticData, Vixen2x',moduleType=MODULE,moduleInstance=MODULE,assemblyName='Module.SequenceType.Vixen2x')
    d=one(module,'{'+NS+'}Vixen2xSequenceStaticData')
    assert text(d,'ModuleInstanceId')==text(d,'ModuleTypeId')==MODULE
    entries=one(d,'{'+NS+'}vixen2xMappings')
    assert len(entries)==1,'Unexpected additional saved map'
    entry=entries[0]
    assert entry.tag=='{'+ARRAYS+'}KeyValueOfstringArrayOfChannelMapping3YEWi_PPm'
    assert text(entry,'{'+ARRAYS+'}Key')==map_name
    records=one(entry,'{'+ARRAYS+'}Value')
    assert len(records)==6 and all(r.tag=='{'+NS+'}ChannelMapping' for r in records)
    rows=[]
    for row in records:
        assert len(row)==7
        mixing=text(row,'{'+NS+'}RgbPixel');assert mixing in ['true','false']
        rows.append(dict(name=text(row,'{'+NS+'}ChannelName'),number=text(row,'{'+NS+'}ChannelNumber'),output=text(row,'{'+NS+'}ChannelOutput'),target=text(row,'{'+NS+'}ElementNodeId'),mixing=mixing=='true',sourceColor=color(row,'ChannelColor'),importColor=color(row,'DestinationColor')))
    for container in root.iter():
        if module in list(container):container.remove(module);break
    return dict(rows=rows,otherXML=canonical(root),moduleStoreSha256=hashlib.sha256(data).hexdigest())

def expected_rows(sample,destinations):
    channels=ET.parse(sample).getroot().findall('Channels/Channel')
    assert len(channels)==6 and len(destinations)==4
    rows=[]
    for i,c in enumerate(channels):
        imported=value(0)
        if i==0:imported=value(0xff00ff00)
        if i==1:imported=dict(knownColor=141,state=1,value=0)
        if i==2:imported=dict(knownColor=37,state=1,value=0)
        rows.append(dict(name=c.attrib['name'],number=str(i+1),output=c.attrib['output'],target=destinations[i] if i<3 else NIL,mixing=False,sourceColor=value(int(c.attrib['color'])&0xffffffff),importColor=imported))
    return rows

def require_saved(store,map_name,expected,initial_xml):
    state=saved_state(store,map_name)
    assert state['rows']==expected,'Saved Vixen2 row target, color, ownership, order, or mixing differs'
    assert state['otherXML']==initial_xml,'Unrelated module store data changed'
    return state


def other_xml(store):
    data=Path(store).read_bytes();assert len(data)<4_000_000
    root=typed_xml(data)
    for parent in root.iter():
        for module in list(parent):
            if module.tag=='Module' and module.attrib.get('assemblyName')=='Module.SequenceType.Vixen2x':parent.remove(module)
    return canonical(root)
