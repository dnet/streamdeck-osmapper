from lxml import etree
from sys import argv
import re

affected = re.compile(r'^[a-z_-]+\.png[1-9][0-9]*$')

t = etree.parse(argv[1])
root = t.getroot()
for e in t.xpath('/osm/node/tag[@k="name"]'):
    if affected.match(e.attrib['v']):
        root.remove(e.getparent())

with open(argv[1] + '.clean.osm', 'wb') as f:
    f.write(etree.tostring(root, pretty_print=True, xml_declaration=True))
