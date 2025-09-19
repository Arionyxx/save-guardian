import os
from PIL import Image, ImageDraw
import io

def create_icon():
    # Create a 256x256 icon
    size = 256
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    center = size // 2
    
    # Background circle (dark blue)
    draw.ellipse([8, 8, size-8, size-8], fill=(44, 62, 80, 255), outline=(52, 73, 94, 255), width=4)
    
    # Shield shape (blue)
    shield_points = [
        (center, 40),      # top
        (90, 60),          # top left
        (90, 120),         # middle left  
        (90, 160),         # bottom left curve start
        (center, 200),     # bottom point
        (166, 160),        # bottom right curve start
        (166, 120),        # middle right
        (166, 60),         # top right
    ]
    draw.polygon(shield_points, fill=(52, 152, 219, 255), outline=(41, 128, 185, 255), width=2)
    
    # Cloud shape (white)
    cloud_y = 90
    # Main cloud body
    draw.ellipse([95, cloud_y-10, 145, cloud_y+10], fill=(255, 255, 255, 230))
    # Cloud bumps
    draw.ellipse([90, cloud_y-8, 110, cloud_y+8], fill=(255, 255, 255, 230))
    draw.ellipse([130, cloud_y-12, 155, cloud_y+8], fill=(255, 255, 255, 230))
    draw.ellipse([105, cloud_y-15, 125, cloud_y+5], fill=(255, 255, 255, 230))
    
    # Save disk icon (white rectangle)
    disk_x = 118
    disk_y = 120
    disk_w = 20
    disk_h = 16
    draw.rounded_rectangle([disk_x, disk_y, disk_x+disk_w, disk_y+disk_h], radius=2, fill=(255, 255, 255, 255))
    
    # Disk details (blue interior)
    draw.rectangle([disk_x+2, disk_y+2, disk_x+disk_w-2, disk_y+disk_h-2], fill=(52, 152, 219, 255))
    
    # Disk lines (white)
    for i in range(3):
        line_y = disk_y + 4 + i * 3
        draw.rectangle([disk_x+4, line_y, disk_x+disk_w-4, line_y+1], fill=(255, 255, 255, 255))
    
    # Shield highlight
    draw.ellipse([112, 55, 128, 85], fill=(255, 255, 255, 51))
    
    return img

def main():
    # Create the main icon
    icon = create_icon()
    
    # Create multiple sizes for ICO format
    sizes = [16, 24, 32, 48, 64, 128, 256]
    images = []
    
    for size in sizes:
        resized = icon.resize((size, size), Image.Resampling.LANCZOS)
        images.append(resized)
    
    # Save as ICO
    icon_path = "assets/icon.ico"
    images[0].save(icon_path, format='ICO', sizes=[(img.width, img.height) for img in images])
    print(f"Icon created successfully: {icon_path}")
    
    # Also save as PNG for GitHub
    icon.save("assets/icon.png", "PNG")
    print("PNG version created: assets/icon.png")

if __name__ == "__main__":
    main()