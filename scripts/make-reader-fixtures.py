from pathlib import Path
from reportlab.pdfgen import canvas
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.lib.pdfencrypt import StandardEncryption
from reportlab.lib.utils import ImageReader
from PIL import Image
import re

root=Path(__file__).resolve().parents[1]/"src-tauri"/"target"
root.mkdir(parents=True,exist_ok=True)
font=Path("C:/Windows/Fonts/Nirmala.ttc")
if font.exists():
    pdfmetrics.registerFont(TTFont("Bangla",str(font)))
def generate(name,pages,encrypted=False):
    doc=canvas.Canvas(str(root/name),pagesize=(432,576),encrypt=StandardEncryption("readerpass") if encrypted else None)
    for page in range(1,pages+1):
        doc.bookmarkPage("page"+str(page))
        if page==1: doc.addOutlineEntry("Chapter one","page1",0)
        if page==3: doc.addOutlineEntry("Nested section","page3",1)
        if page==5: doc.addOutlineEntry("Chapter two","page5",0)
        doc.setFont("Helvetica",18)
        doc.drawString(32,520,"Reader fixture page "+str(page))
        doc.setFont("Helvetica",12)
        doc.drawString(32,480,"Needle matching text. English chapter.")
        if font.exists():
            doc.setFont("Bangla",14)
            doc.drawString(32,440,"বাংলা বই যুক্তাক্ষর")
        if name=="reader-fixture.pdf" and page==10:
            image=Image.effect_noise((1600,1600),100).convert("RGB")
            doc.drawImage(ImageReader(image),32,160,width=360,height=260)
        doc.showPage()
    doc.save()
generate("reader-fixture.pdf",30)
generate("reader-long.pdf",2000)
generate("reader-locked.pdf",6,True)

c=canvas.Canvas(str(root/"reader-blank.pdf"));c.showPage();c.save()
c=canvas.Canvas(str(root/"reader-text-limit.pdf"))
for i in range(10000): c.drawString(30,300,"Text content size limit verification 0123456789")
c.showPage();c.save()
(root/"reader-invalid.pdf").write_bytes(b"%PDF-1.7\nmalformed fixture")
data=(root/"reader-fixture.pdf").read_bytes()
xref=data.rfind(b"\nxref\n")+1
padding=300*1024*1024
tail=re.sub(rb"(startxref\s+)\d+",lambda m:m.group(1)+str(xref+padding).encode(),data[xref:])
with (root/"reader-large.pdf").open("wb") as out:
    out.write(data[:xref])
    out.seek(xref+padding)
    out.write(tail)
print("Created reader fixtures, including", (root/"reader-large.pdf").stat().st_size,"bytes and 2,000 pages")
