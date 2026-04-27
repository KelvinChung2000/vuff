module m;
  initial begin
    a = {<<{x, y, z}};
    b = {>>{a, b, c}};
    c = {<<8{data}};
    d = {>>{a, b, {c, d}}};
  end
endmodule
// expected -----
module m;
  initial begin
    a = {<<{x, y, z}};
    b = {>>{a, b, c}};
    c = {<<8{data}};
    d = {>>{a, b, {c, d}}};
  end
endmodule
