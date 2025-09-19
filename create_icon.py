from PIL import Image, ImageDraw
import os

def create_icon():
    # Create a 256x256 icon
    size = 256
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    center = size // 2
    
    # Background circle (dark blue)
    draw.ellipse([8, 8, size-8, size-8], fill=(44, 62, 80, 255), outline=(52, 73, 94, 255), width=4)
    
    # Shield shape (blue) - create a proper shield
    shield_width = 76
    shield_height = 160
    shield_left = center - shield_width // 2
    shield_right = center + shield_width // 2
    shield_top = 40
    shield_bottom = shield_top + shield_height
    
    # Shield polygon points
    shield_points = [
        (center, shield_top),           # top point
        (shield_left, shield_top + 20), # top left
        (shield_left, shield_top + 80), # middle left
        (shield_left, shield_bottom - 40), # bottom left curve start
        (center, shield_bottom),        # bottom point
        (shield_right, shield_bottom - 40), # bottom right curve start
        (shield_right, shield_top + 80), # middle right
        (shield_right, shield_top + 20), # top right
    ]
    
    draw.polygon(shield_points, fill=(52, 152, 219, 255), outline=(41, 128, 185, 255), width=3)
    
    # Cloud shape (white) - make it more cloud-like
    cloud_y = 100
    cloud_center = center
    
    # Main cloud circles
    draw.ellipse([cloud_center-30, cloud_y-10, cloud_center+30, cloud_y+10], fill=(255, 255, 255, 240))
    draw.ellipse([cloud_center-25, cloud_y-15, cloud_center+15, cloud_y+5], fill=(255, 255, 255, 240))
    draw.ellipse([cloud_center-10, cloud_y-12, cloud_center+25, cloud_y+8], fill=(255, 255, 255, 240))
    draw.ellipse([cloud_center-15, cloud_y-8, cloud_center+20, cloud_y+12], fill=(255, 255, 255, 240))
    
    # Save disk icon (white rectangle) - make it more detailed
    disk_x = center - 12
    disk_y = 140
    disk_w = 24
    disk_h = 20
    
    # Outer disk (silver/white)
    draw.rounded_rectangle([disk_x, disk_y, disk_x+disk_w, disk_y+disk_h], 
                          radius=3, fill=(240, 240, 240, 255), outline=(200, 200, 200, 255), width=1)
    
    # Inner disk (blue)
    draw.rectangle([disk_x+3, disk_y+3, disk_x+disk_w-3, disk_y+disk_h-3], fill=(52, 152, 219, 255))
    
    # Disk label area (white)
    draw.rectangle([disk_x+3, disk_y+3, disk_x+disk_w-3, disk_y+8], fill=(255, 255, 255, 255))
    
    # Disk data lines (white)
    for i in range(3):
        line_y = disk_y + 10 + i * 3
        if line_y < disk_y + disk_h - 3:
            draw.rectangle([disk_x+6, line_y, disk_x+disk_w-6, line_y+1], fill=(255, 255, 255, 255))
    
    # Shield highlight for 3D effect
    highlight_points = [
        (center-5, shield_top+10),
        (shield_left+10, shield_top+25),
        (shield_left+10, shield_top+60),
        (center-15, shield_top+80),
    ]
    draw.polygon(highlight_points, fill=(255, 255, 255, 60))
    
    return img

def main():
    print("Creating Save Guardian icon...")
    
    # Create the main icon
    icon = create_icon()
    
    # Create multiple sizes for ICO format
    sizes = [16, 24, 32, 48, 64, 128, 256]
    images = []
    
    for size in sizes:
        resized = icon.resize((size, size), Image.Resampling.LANCZOS)
        images.append(resized)
    
    # Save as ICO with all sizes
    icon_path = "assets/icon.ico"
    print(f"Saving ICO with sizes: {[img.size for img in images]}")
    
    # Use the largest image as the base and include all sizes
    icon.save(icon_path, format='ICO', sizes=[(img.width, img.height) for img in images])
    print(f"✅ Icon created successfully: {icon_path}")
    
    # Also save as PNG for GitHub/documentation  
    icon.save("assets/icon.png", "PNG")
    print("✅ PNG version created: assets/icon.png")
    
    # Verify the ICO file
    try:
        test_icon = Image.open(icon_path)
        print(f"✅ ICO file verification successful: {test_icon.size}, format: {test_icon.format}")
    except Exception as e:
        print(f"❌ ICO file verification failed: {e}")

if __name__ == "__main__":
    main()