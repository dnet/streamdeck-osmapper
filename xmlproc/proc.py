from lxml import etree
from sys import argv

t = etree.parse(argv[1])
root = t.getroot()
for e in t.xpath('/osm/node/tag[@k="fixme"]'):
    if e.attrib['v'].startswith('streamdeck-osmapper #'):
        root.remove(e.getparent())

with open(argv[1] + '.clean.osm', 'wb') as f:
    f.write(etree.tostring(root, pretty_print=True, xml_declaration=True))
